use std::error::Error;
use std::fmt::Display;
use std::future::Future;
use std::ops::Deref;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thirtyfour::extensions::cdp::ChromeDevTools;
use thirtyfour::prelude::{By, DesiredCapabilities, WebDriver, WebDriverError, WebDriverResult};
use tokio::sync::Mutex;
use tokio::time::sleep;

#[derive(Deserialize, Serialize, Debug)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: f64,
}

#[derive(Debug)]
pub struct LocationInfo {
    pub name: String,
    pub url: String,
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
    TimeoutError(String, Option<Box<ScrapeError>>),
}

impl From<WebDriverError> for ScrapeError {
    fn from(value: WebDriverError) -> Self {
        ScrapeError::WebDriverError(value)
    }
}

impl Display for ScrapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScrapeError::WebDriverError(e) => write!(f, "WebDriverError({})", e),
            ScrapeError::ParseError(e) => write!(f, "ParseError({})", e),
            ScrapeError::TimeoutError(e, Some(x)) => write!(f, "TimeoutError({}, {})", e, x),
            ScrapeError::TimeoutError(e, None) => write!(f, "TimeoutError({})", e),
        }
    }
}

impl Error for ScrapeError {}

impl ScrapeError {
    pub fn timeout<S: Display>(msg: S, inner: Option<ScrapeError>) -> Self {
        ScrapeError::TimeoutError(format!("{}", msg), inner.map(|x| Box::new(x)))
    }
}

pub struct Client {
    driver: Mutex<(WebDriver, ChromeDevTools)>,
}

impl Client {
    pub async fn new(driver: &str) -> anyhow::Result<Client> {
        let caps = DesiredCapabilities::chrome();
        let driver = WebDriver::new(driver, caps).await?;
        let tools = ChromeDevTools::new(driver.handle.clone());
        Ok(Client {
            driver: Mutex::new((driver, tools)),
        })
    }

    pub async fn search(
        &self,
        search: &str,
        location: &GeoLocation,
    ) -> Result<SearchResult, ScrapeError> {
        let unlocked = self.driver.lock().await;
        let (driver, dev_tools) = unlocked.deref();
        set_location(dev_tools, location).await?;
        driver.goto("https://www.google.com/maps").await?;
        let query = driver.find(By::Name("q")).await?;
        query.focus().await?;
        query.send_keys(search).await?;
        query.send_keys("\n").await?;

        Ok(wait_for_scrape_result(driver, decode_search_result).await?)
    }

    pub async fn list_reviews(&self, url: &str, location: &GeoLocation) -> Result<(), ScrapeError> {
        let unlocked = self.driver.lock().await;
        let (driver, dev_tools) = unlocked.deref();
        set_location(dev_tools, location).await?;
        driver.goto(url).await?;
        wait_for_scrape_result(driver, click_more_reviews_button).await?;
        // TODO: parse results.
        Ok(())
    }

    pub async fn close(&self) -> WebDriverResult<()> {
        self.driver.lock().await.deref().0.close_window().await
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
                return Err(ScrapeError::ParseError(
                    "missing expected area-label on main content".to_owned(),
                ));
            }
        }
        return Err(ScrapeError::ParseError(
            "no main content was found".to_owned(),
        ));
    }

    // Look for the string indicating no results are found.
    for x in driver.find_all(By::Tag("div")).await? {
        if x.text().await?.starts_with("Google Maps can't find") {
            return Ok(SearchResult::NotFound);
        }
    }

    // Look for an indication that multiple results were found.
    for x in driver
        .find_all(By::XPath("//*[starts-with(@aria-label, 'Results for')]"))
        .await?
    {
        let mut destinations: Vec<LocationInfo> = Vec::new();
        for link in x.find_all(By::Tag("a")).await? {
            if let Some(href) = link.attr("href").await? {
                if let Some(name) = link.attr("aria-label").await? {
                    destinations.push(LocationInfo {
                        name: name,
                        url: href,
                    });
                }
            }
        }
        return Ok(SearchResult::Multiple(destinations));
    }

    Err(ScrapeError::ParseError(
        "unable to parse search results".to_owned(),
    ))
}

async fn click_more_reviews_button(driver: &WebDriver) -> Result<(), ScrapeError> {
    for x in driver
        .find_all(By::XPath(
            "//*[starts-with(@jsaction, 'pane.reviewChart.moreReviews')]",
        ))
        .await?
    {
        x.click().await?;
        return Ok(());
    }
    return Err(ScrapeError::ParseError(
        "no 'more reviews' button found".to_owned(),
    ));
}
