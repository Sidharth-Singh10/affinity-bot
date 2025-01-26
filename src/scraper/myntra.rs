use headless_chrome::{Browser, LaunchOptionsBuilder};
use rand::Rng;
use scraper::{Html, Selector};
use std::{ffi::OsStr, thread, time::Duration};

pub async fn scrape_product(url: &str) -> Result<f32, Box<dyn std::error::Error>> {
    let options = LaunchOptionsBuilder::default()
        .args(vec![
            OsStr::new("--disable-blink-features=AutomationControlled"),
             OsStr::new("--disable-gpu"),
             OsStr::new("--no-sandbox"),
             OsStr::new("--window-size=1920,1080"),
             OsStr::new("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        ])
        .headless(true)
        .build()?;

    let browser = Browser::new(options)?;
    let tab = browser.new_tab()?;

    // Initial random delay
    let delay = rand::thread_rng().gen_range(2000..5000);
    thread::sleep(Duration::from_millis(delay));

    // Hide automation
    tab.evaluate(
        r#"
        Object.defineProperty(navigator, 'webdriver', {
            get: () => false,
        });
    "#,
        true,
    )?;

    // Set custom headers before navigation
    tab.evaluate(&format!(r#"
        // Override fetch to add our headers
        const originalFetch = window.fetch;
        window.fetch = function(input, init) {{
            const headers = {{
                'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8',
                'Accept-Language': 'en-US,en;q=0.9',
                'Cache-Control': 'max-age=0',
                'Connection': 'keep-alive',
                'Sec-Ch-Ua': '"Not_A Brand";v="8", "Chromium";v="120"',
                'Sec-Ch-Ua-Mobile': '?0',
                'Sec-Ch-Ua-Platform': '"Windows"',
                'Upgrade-Insecure-Requests': '1',
                'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
                'Sec-Fetch-Dest': 'document',
                'Sec-Fetch-Mode': 'navigate',
                'Sec-Fetch-Site': 'same-origin',
                'Sec-Fetch-User': '?1'
            }};
            init = init || {{}};
            init.headers = {{ ...headers, ...init.headers }};
            return originalFetch(input, init);
        }};
    "#), true)?;

    let url2 = format!("https://www.myntra.com/{}", url);
    tab.navigate_to(&url2)?;

    tab.wait_until_navigated()?;

    // Simulate human behavior
    tab.evaluate(
        r#"
        function simulateHumanBehavior() {
            window.scrollTo(0, Math.random() * 100);
            setTimeout(() => {
                window.scrollTo(0, Math.random() * 500);
            }, 1000);
        }
        simulateHumanBehavior();
    "#,
        true,
    )?;

    let page_content = tab.get_content()?;

    let document = Html::parse_document(&page_content);

    let price_selector = Selector::parse("span.pdp-price").unwrap();

    let price = document
        .select(&price_selector)
        .next()
        .and_then(|el| {
            let price_text = el.text().collect::<String>();

            Some(
                price_text
                    .trim()
                    .replace("MRP₹", "")
                    .replace("₹", "")
                    .replace(',', "")
                    .trim()
                    .parse::<f32>()
                    .unwrap_or(0.0),
            )
        })
        .unwrap_or(0.0);

    Ok(price)
}
