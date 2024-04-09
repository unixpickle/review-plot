use std::ops::Deref;

use serde::{Deserialize, Serialize};
use thirtyfour::extensions::cdp::ChromeDevTools;
use thirtyfour::prelude::{By, DesiredCapabilities, WebDriver, WebDriverResult};
use tokio::sync::Mutex;

#[derive(Deserialize, Serialize)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: f64,
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
    ) -> anyhow::Result<Vec<String>> {
        let unlocked = self.driver.lock().await;
        let (driver, dev_tools) = unlocked.deref();
        set_location(dev_tools, location).await?;
        driver.goto("https://www.google.com/maps").await?;
        let query = driver.find(By::Name("q")).await?;
        query.focus().await?;
        query.send_keys(search).await?;
        query.send_keys("\n").await?;

        // TODO: wait for results, parse them.

        Ok(vec![])
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
