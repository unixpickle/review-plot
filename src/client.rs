use std::error::Error;
use std::fmt::Display;
use std::future::Future;
use std::mem::take;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thirtyfour::extensions::cdp::ChromeDevTools;
use thirtyfour::prelude::{By, DesiredCapabilities, WebDriver, WebDriverError, WebDriverResult};
use tokio::time::sleep;

#[derive(Deserialize, Serialize, Debug)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocationInfo {
    pub name: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Review {
    pub timestamp: f64,
    pub author: String,
    pub content: String,
    pub rating: f64,
}

#[derive(Debug, Default)]
struct ReviewResult {
    pub is_final: bool,
    pub reviews: Vec<Review>,
}

#[derive(Debug)]
pub enum SearchResult {
    Singular(LocationInfo),
    Multiple(Vec<LocationInfo>),
    NotFound,
}

#[derive(Debug)]
pub enum ScrapeError {
    WebDriverError(WebDriverError),
    ParseError(String),
    FatalParseError(String),
    TimeoutError(String, Option<Box<ScrapeError>>),
    JsonError(serde_json::Error),
}

impl From<WebDriverError> for ScrapeError {
    fn from(value: WebDriverError) -> Self {
        ScrapeError::WebDriverError(value)
    }
}

impl From<serde_json::Error> for ScrapeError {
    fn from(value: serde_json::Error) -> Self {
        ScrapeError::JsonError(value)
    }
}

impl Display for ScrapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScrapeError::WebDriverError(e) => write!(f, "WebDriverError({})", e),
            ScrapeError::ParseError(e) => write!(f, "ParseError({})", e),
            ScrapeError::FatalParseError(e) => write!(f, "FatalParseError({})", e),
            ScrapeError::TimeoutError(e, Some(x)) => write!(f, "TimeoutError({}, {})", e, x),
            ScrapeError::TimeoutError(e, None) => write!(f, "TimeoutError({})", e),
            ScrapeError::JsonError(e) => write!(f, "JsonError({})", e),
        }
    }
}

impl Error for ScrapeError {}

impl ScrapeError {
    pub fn timeout<S: Display>(msg: S, inner: Option<ScrapeError>) -> Self {
        ScrapeError::TimeoutError(format!("{}", msg), inner.map(|x| Box::new(x)))
    }

    pub fn parse_error<S: Display>(msg: S) -> Self {
        ScrapeError::ParseError(format!("{}", msg))
    }

    pub fn fatal_parse_error<S: Display>(msg: S) -> Self {
        ScrapeError::FatalParseError(format!("{}", msg))
    }
}

pub struct ReviewIter<'a> {
    client: &'a mut Client,
    first: ReviewResult,
    on_first: bool,
    done: bool,
}

impl<'a> ReviewIter<'a> {
    fn new(client: &'a mut Client, first: ReviewResult) -> Self {
        ReviewIter {
            client: client,
            first: first,
            on_first: true,
            done: false,
        }
    }

    pub async fn next(&mut self) -> Result<Option<Vec<Review>>, ScrapeError> {
        if self.done {
            Ok(None)
        } else if self.on_first {
            self.on_first = false;
            let first = take(&mut self.first);
            if first.is_final {
                self.done = true;
            }
            Ok(Some(first.reviews))
        } else {
            let next = self.client.next_reviews().await?;
            if next.is_final {
                self.done = true;
            }
            Ok(Some(next.reviews))
        }
    }
}

pub struct Client {
    driver: WebDriver,
    dev_tools: ChromeDevTools,
}

impl Client {
    pub async fn new(driver: &str) -> WebDriverResult<Client> {
        let caps = DesiredCapabilities::chrome();
        let driver = WebDriver::new(driver, caps).await?;
        let tools = ChromeDevTools::new(driver.handle.clone());
        Ok(Client {
            driver: driver,
            dev_tools: tools,
        })
    }

    pub async fn search(
        &self,
        search: &str,
        location: &GeoLocation,
    ) -> Result<SearchResult, ScrapeError> {
        set_location(&self.dev_tools, location).await?;
        self.driver.goto("https://www.google.com/maps").await?;
        let query = self.driver.find(By::Name("q")).await?;
        query.focus().await?;
        query.send_keys(search).await?;
        query.send_keys("\n").await?;

        Ok(wait_for_scrape_result(&self.driver, decode_search_result).await?)
    }

    pub async fn list_reviews<'a>(
        &'a mut self,
        url: &str,
        location: &GeoLocation,
    ) -> Result<ReviewIter<'a>, ScrapeError> {
        set_location(&self.dev_tools, location).await?;

        // Intentionally clear any scripts on the page.
        self.driver.goto("https://google.com").await?;
        self.driver.goto(url).await?;

        // Load script that will dump all requests.
        self.driver
            .execute(
                r#"
                    const origOpen = XMLHttpRequest.prototype.open;
                    XMLHttpRequest.prototype.open = function(method, url) {
                        this._url = url;
                        return origOpen.apply(this, arguments);
                    };
                    const origSend = XMLHttpRequest.prototype.send;
                    window.recordedReviewResponses = [];
                    XMLHttpRequest.prototype.send = function() {
                        const oldCb = this.onreadystatechange;
                        this.onreadystatechange = function() {
                            if (this.readyState == 4 && this._url.includes('listugcposts')) {
                                window.recordedReviewResponses.push(this.response);
                            }
                            if (oldCb) {
                                return oldCb.apply(this, arguments);
                            }
                        };
                        origSend.apply(this, arguments);
                    }
                "#,
                vec![],
            )
            .await?;
        wait_for_scrape_result(&self.driver, click_more_reviews_button).await?;

        let reviews = wait_for_scrape_result(&self.driver, get_logged_reviews).await?;
        Ok(ReviewIter::new(self, reviews))
    }

    async fn next_reviews(&self) -> Result<ReviewResult, ScrapeError> {
        self.driver
            .execute("window.recordedReviewResponses = [];", vec![])
            .await?;
        let scroll_code = r#"
            const allDivs = Array.from(document.getElementsByTagName('div'));
            const reviewItems = allDivs.filter((x) => !!x.getAttribute('data-review-id'));
            const reviewContainer = reviewItems[0].parentElement.parentElement;
            reviewContainer.scrollTop = 10000000;
        "#;
        self.driver.execute(scroll_code, vec![]).await?;
        if let Ok(next) = wait_for_scrape_result(&self.driver, get_logged_reviews).await {
            Ok(next)
        } else {
            // Try to scroll one more time, since sometimes
            // images took a while to load and prevented the
            // scroll from triggering a reload.
            self.driver.execute(scroll_code, vec![]).await?;
            wait_for_scrape_result(&self.driver, get_logged_reviews).await
        }
    }

    pub async fn close(self) -> WebDriverResult<()> {
        self.driver.close_window().await
    }
}

async fn set_location(dev_tools: &ChromeDevTools, location: &GeoLocation) -> WebDriverResult<()> {
    dev_tools
        .execute_cdp_with_params(
            "Emulation.setGeolocationOverride",
            serde_json::to_value(location)?,
        )
        .await
        .and_then(|_| Ok(()))
}

async fn wait_for_scrape_result<'a, T, Fut, F>(
    driver: &'a WebDriver,
    f: F,
) -> Result<T, ScrapeError>
where
    Fut: Future<Output = Result<T, ScrapeError>>,
    F: Fn(&'a WebDriver) -> Fut,
{
    let mut last_error: Option<ScrapeError> = None;
    for _ in 0..10 {
        match f(driver).await {
            Ok(result) => return Ok(result),
            Err(ScrapeError::WebDriverError(WebDriverError::StaleElementReference(_))) => {}
            Err(ScrapeError::WebDriverError(x)) => return Err(x.into()),
            Err(e @ ScrapeError::FatalParseError(_)) => return Err(e),
            Err(x) => last_error = Some(x),
        }
        sleep(Duration::from_secs(1)).await;
    }
    Err(ScrapeError::timeout(
        "timeout while waiting for results",
        last_error,
    ))
}

async fn decode_search_result(driver: &WebDriver) -> Result<SearchResult, ScrapeError> {
    // See if we are looking at a single result.
    let current_url = driver.current_url().await?.to_string();
    if current_url.contains("/maps/place") {
        for x in driver
            .find_all(By::XPath("//*[starts-with(@role, 'main')]"))
            .await?
        {
            if let Some(name) = x.attr("aria-label").await? {
                return Ok(SearchResult::Singular(LocationInfo {
                    name: name,
                    url: current_url,
                }));
            } else {
                return Err(ScrapeError::parse_error(
                    "missing expected area-label on main content",
                ));
            }
        }
        return Err(ScrapeError::parse_error("no main content was found"));
    }

    let no_results: bool = driver
        .execute(
            "
            const divs = document.getElementsByTagName('div');
            for (let i = 0; i < divs.length; i++) {
                if (divs[i].textContent.startsWith('Google Maps can\\'t find')) {
                    return true;
                }
            }
            return false;
            ",
            vec![],
        )
        .await?
        .convert()?;
    if no_results {
        return Ok(SearchResult::NotFound);
    }

    // Look for an indication that multiple results were found.
    let destinations: Vec<LocationInfo> = driver
        .execute(
            "
            const divs = document.getElementsByTagName('div');
            const results = [];
            for (let i = 0; i < divs.length; i++) {
                const div = divs[i];
                if ((div.getAttribute('aria-label') || '').startsWith('Results for')) {
                    const links = div.getElementsByTagName('a');
                    for (let j = 0; j < links.length; j++) {
                        const link = links[j];
                        const href = link.href;
                        const name = link.getAttribute('aria-label');
                        if (href && name) {
                            results.push({name: name, url: href});
                        }
                    }
                }
            }
            return results;
            ",
            vec![],
        )
        .await?
        .convert()?;

    if destinations.len() > 0 {
        Ok(SearchResult::Multiple(destinations))
    } else {
        Err(ScrapeError::parse_error("unable to parse search results"))
    }
}

async fn click_more_reviews_button(driver: &WebDriver) -> Result<(), ScrapeError> {
    // Click the 'more reviews' button even if it's offscreen by using
    // javascript instead of the click() function.
    let result: bool = driver
        .execute(
            r#"
                let buttons = Array.from(document.getElementsByTagName('button')).filter((x) => {
                    const attr = x.getAttribute('jsaction');
                    return attr && attr.endsWith('reviewChart.moreReviews');
                });
                if (buttons.length) {
                    buttons[0].click();
                    return true;
                } else {
                    return false;
                }
            "#,
            vec![],
        )
        .await?
        .convert()?;
    if result {
        Ok(())
    } else {
        Err(ScrapeError::parse_error("no 'more reviews' button found"))
    }
}

async fn get_logged_reviews(driver: &WebDriver) -> Result<ReviewResult, ScrapeError> {
    let result = driver
        .execute("return window.recordedReviewResponses", vec![])
        .await?;
    let results: Vec<String> = result.convert()?;
    let mut is_final: bool = false;
    if results.len() != 0 {
        let mut parsed = Vec::new();
        for result in results {
            let parsed_result = parse_logged_reviews(&result)?;
            if parsed_result.is_final {
                is_final = true;
            }
            parsed.extend(parsed_result.reviews);
        }
        return Ok(ReviewResult {
            is_final: is_final,
            reviews: parsed,
        });
    }
    return Err(ScrapeError::parse_error(
        "did not find any review HTTP requests",
    ));
}

fn parse_logged_reviews(response: &str) -> Result<ReviewResult, ScrapeError> {
    let last_line = response
        .split('\n')
        .last()
        .ok_or_else(|| ScrapeError::fatal_parse_error("expected newline in reviews"))?;
    let results: serde_json::Value = serde_json::from_str(last_line)?;
    let items = as_array("root list", &results)?;
    let mut reviews = Vec::new();
    let is_final: bool = items[1].is_null();
    for (i, x) in items.into_iter().enumerate() {
        if x.is_null() || x.is_string() {
            continue;
        }
        let review_lists = as_array(
            format!(
                "root index {} should be array, string, or null; got {:?}",
                i, x
            ),
            x,
        )?;
        for (i, x) in review_lists.into_iter().enumerate() {
            let data_list = get_array_index(
                &format!("review list entry {} should be array with a value", i),
                x,
                0,
            )?;
            let data_list_err = format!("review list entry {} has bad data list", i);
            let review_metadata = get_array_index(&data_list_err, data_list, 1)?;
            let metadata_err = format!("review list entry {} has bad metadata", i);
            let review_timestamp = as_number(
                &metadata_err,
                get_array_index(&metadata_err, review_metadata, 2)?,
            )?;
            let review_author = as_string(
                &metadata_err,
                get_array_index(
                    &metadata_err,
                    get_array_index(
                        &metadata_err,
                        get_array_index(&metadata_err, review_metadata, 4)?,
                        0,
                    )?,
                    4,
                )?,
            )?
            .to_owned();
            let review_content = get_array_index(&data_list_err, data_list, 2)?;
            let star_err = format!("review list entry {} invalid stars", i);
            let review_stars = if get_array_index(&star_err, review_content, 0)?.is_null() {
                // This is for reviews from other sites, where we have an object at index
                // 8 that looks like [null,4,"4/5","0"].
                //
                // Alternatively looks like [BUNCH_OF_DATA,8,"8/10","0"].
                // We want to support reviews that are out of any scale, so we parse the
                // divisor in the third entry.
                let divisor: f64 = as_string(
                    &star_err,
                    get_array_index(&star_err, get_array_index(&star_err, review_content, 8)?, 2)?,
                )?
                .split("/")
                .last()
                .ok_or_else(|| ScrapeError::parse_error("failed to identify review scale"))?
                .parse()
                .map_err(|e| ScrapeError::parse_error(format!("invalid review scale: {}", e)))?;

                ((5.0 / divisor)
                    * as_number(
                        &star_err,
                        get_array_index(
                            &star_err,
                            get_array_index(&star_err, review_content, 8)?,
                            1,
                        )?,
                    )?)
                .clamp(1.0, 5.0)
            } else {
                as_number(
                    &star_err,
                    get_array_index(&star_err, get_array_index(&star_err, review_content, 0)?, 0)?,
                )?
            };
            let review_text_container = get_array_index(
                &format!("review list entry {} invalid text", i),
                review_content,
                -1,
            )?;
            let text_err = format!(
                "review list entry {} invalid text: {}",
                i, review_text_container,
            );
            let review_text = if get_array_index(&text_err, review_text_container, 0)?.is_string() {
                // Sometimes an empty review's text element is just ["en"] instead of containing
                // the actual review text.
                "".to_owned()
            } else {
                // We ignore errors here because there are a few different types
                // of reviews. By default we will get some text, but we could also
                // get a review_text_container like this:
                //
                //     "[[[\"GUIDED_DINING_MODE\"],\"Did you dine in, take out, or get delivery?\",[[[[\"E:DINE_IN\"],\"Dine in\",2,null,null,\"0ahUKEwip-ama_NOFAxWoClcBHdn9BlYQ3YcHCDUoAA\",null,null,0]],1],null,null,\"Service\",null,\"0ahUKEwip-ama_NOFAxWoClcBHdn9BlYQ3IcHCDQoBw\",null,null,null,null,null,1],[[\"GUIDED_DINING_MEAL_TYPE\"],\"What did you get?\",[[[[\"E:LUNCH\"],\"Lunch\",2,null,null,\"0ahUKEwip-ama_NOFAxWoClcBHdn9BlYQ3YcHCDcoAA\",null,null,0]],1],null,null,\"Meal type\",null,\"0ahUKEwip-ama_NOFAxWoClcBHdn9BlYQ3IcHCDYoCA\",null,null,null,null,null,1],[[\"GUIDED_DINING_PRICE_RANGE\"],\"How much did you spend per person?\",[[[[\"E:USD_30_TO_50\"],\"$30–50\",2,null,\"$30 to $50\",\"0ahUKEwip-ama_NOFAxWoClcBHdn9BlYQ3YcHCDkoAA\"]],1],null,null,\"Price per person\",null,\"0ahUKEwip-ama_NOFAxWoClcBHdn9BlYQ3IcHCDgoCQ\",null,null,null,null,null,1,[[2]]]]"
                //
                // Or one like this: "[4]"
                || -> Result<String, ScrapeError> {
                    Ok(as_string(
                        &text_err,
                        get_array_index(
                            &text_err,
                            get_array_index(&text_err, review_text_container, 0)?,
                            0,
                        )?,
                    )?
                    .to_owned())
                }()
                .unwrap_or_default()
            };
            reviews.push(Review {
                timestamp: review_timestamp / 1000000.0,
                author: review_author,
                content: review_text,
                rating: review_stars,
            });
        }
    }
    Ok(ReviewResult {
        is_final: is_final,
        reviews: reviews,
    })
}

fn as_string<D: Display>(err_ctx: D, x: &serde_json::Value) -> Result<&str, ScrapeError> {
    if let serde_json::Value::String(x) = x {
        Ok(x)
    } else {
        Err(ScrapeError::FatalParseError(format!(
            "expected JSON string: {}",
            err_ctx
        )))
    }
}

fn as_number<D: Display>(err_ctx: D, x: &serde_json::Value) -> Result<f64, ScrapeError> {
    if let serde_json::Value::Number(x) = x {
        Ok(x.as_f64().unwrap_or_default())
    } else {
        Err(ScrapeError::FatalParseError(format!(
            "expected JSON string: {}",
            err_ctx
        )))
    }
}

fn as_array<D: Display>(
    err_ctx: D,
    x: &serde_json::Value,
) -> Result<&[serde_json::Value], ScrapeError> {
    if let serde_json::Value::Array(x) = x {
        Ok(x)
    } else {
        Err(ScrapeError::FatalParseError(format!(
            "expected JSON array: {}",
            err_ctx
        )))
    }
}

fn get_array_index<'a, D: Display>(
    err_ctx: &D,
    val: &'a serde_json::Value,
    index: i32,
) -> Result<&'a serde_json::Value, ScrapeError> {
    let in_list = as_array(err_ctx, val)?;
    let i = if index < 0 {
        index + (in_list.len() as i32)
    } else {
        index
    };
    if i >= in_list.len() as i32 {
        return Err(ScrapeError::FatalParseError(format!(
            "array index {} out of bounds: {}",
            i, err_ctx
        )));
    }
    Ok(&in_list[i as usize])
}
