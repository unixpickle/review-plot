use std::ops::Deref;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thirtyfour::extensions::cdp::ChromeDevTools;
use thirtyfour::prelude::{By, DesiredCapabilities, WebDriver, WebDriverResult};
use tokio::sync::Mutex;
use tokio::time::sleep;

#[derive(Deserialize, Serialize)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: f64,
}

#[derive(Debug)]
pub enum SearchResult {
    Singular(String),
    Multiple(Vec<String>),
    NotFound,
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
    ) -> anyhow::Result<SearchResult> {
        let unlocked = self.driver.lock().await;
        let (driver, dev_tools) = unlocked.deref();
        set_location(dev_tools, location).await?;
        driver.goto("https://www.google.com/maps").await?;
        let query = driver.find(By::Name("q")).await?;
        query.focus().await?;
        query.send_keys(search).await?;
        query.send_keys("\n").await?;

        for _ in 0..5 {
            if let Some(result) = decode_search_result(driver).await? {
                return Ok(result);
            }
            sleep(Duration::from_secs(1)).await;
        }

        Err(anyhow::format_err!("timeout while fetching results"))
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

async fn decode_search_result(driver: &WebDriver) -> WebDriverResult<Option<SearchResult>> {
    // See if we are looking at a single result.
    let current_url = driver.current_url().await?.to_string();
    if current_url.contains("/maps/place") {
        return Ok(Some(SearchResult::Singular(current_url)));
    }

    // Look for the string indicating no results are found.
    for x in driver.find_all(By::Tag("div")).await? {
        if x.text().await?.starts_with("Google Maps can't find") {
            return Ok(Some(SearchResult::NotFound));
        }
    }

    // Look for an indication that multiple results were found.
    for x in driver
        .find_all(By::XPath("//*[starts-with(@aria-label, 'Results for')]"))
        .await?
    {
        let mut destinations: Vec<String> = Vec::new();
        for link in x.find_all(By::Tag("a")).await? {
            let href = link.attr("href").await?;
            if let Some(href) = href {
                destinations.push(href);
            }
        }
        return Ok(Some(SearchResult::Multiple(destinations)));
    }

    Ok(None)
}
