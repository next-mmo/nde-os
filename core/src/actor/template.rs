use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::manifest::*;
use crate::shield::browser::BrowserEngine;

// ─── Template Types ────────────────────────────────────────────────

/// Built-in actor templates for scaffolding new actors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActorTemplate {
    /// Basic web scraping with Shield anti-detect browser.
    ShieldScraper,
    /// Multi-page crawling with URL queue and depth control.
    ShieldCrawler,
    /// Screenshot actor — takes URLs, produces PNG screenshots.
    ShieldScreenshot,
    /// Form automation with Shield profile (preserves session).
    ShieldFormFiller,
    /// Social media automation template.
    ShieldSocialBot,
    /// Android emulator automation via ADB.
    EmulatorBot,
    /// LDPlayer automation — open URL and grab website title.
    LdPlayerTitleGrabber,
}

/// Template metadata for listing in the UI.
#[derive(Debug, Clone, Serialize)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub tags: Vec<String>,
    pub runtime: String,
}

impl ActorTemplate {
    /// List all available templates with metadata.
    pub fn all() -> Vec<TemplateInfo> {
        vec![
            TemplateInfo {
                id: "shield_scraper".into(),
                name: "Shield Scraper".into(),
                description: "Extract data from websites using Shield anti-detect browser. Auto-spoofs fingerprints to avoid detection.".into(),
                icon: "🕷️".into(),
                tags: vec!["scraping".into(), "anti-detect".into(), "data-extraction".into()],
                runtime: "python".into(),
            },
            TemplateInfo {
                id: "shield_crawler".into(),
                name: "Shield Crawler".into(),
                description: "Crawl multi-page websites with URL queue, depth control, and anti-detect protection.".into(),
                icon: "🔗".into(),
                tags: vec!["crawling".into(), "anti-detect".into(), "deep-scrape".into()],
                runtime: "python".into(),
            },
            TemplateInfo {
                id: "shield_screenshot".into(),
                name: "Shield Screenshot".into(),
                description: "Take screenshots of URLs with custom viewport, full-page capture, and anti-detect browser.".into(),
                icon: "📸".into(),
                tags: vec!["screenshot".into(), "visual".into(), "monitoring".into()],
                runtime: "python".into(),
            },
            TemplateInfo {
                id: "shield_form_filler".into(),
                name: "Shield Form Filler".into(),
                description: "Automate form filling and submission with persistent Shield profiles (preserves cookies, sessions).".into(),
                icon: "📝".into(),
                tags: vec!["forms".into(), "automation".into(), "sessions".into()],
                runtime: "python".into(),
            },
            TemplateInfo {
                id: "shield_social_bot".into(),
                name: "Shield Social Bot".into(),
                description: "Social media automation with anti-detect fingerprints, proxy rotation, and session persistence.".into(),
                icon: "🤖".into(),
                tags: vec!["social".into(), "anti-detect".into(), "automation".into()],
                runtime: "python".into(),
            },
            TemplateInfo {
                id: "emulator_bot".into(),
                name: "Emulator Bot".into(),
                description: "Android emulator automation via ADB. Control mobile apps, take screenshots, configure proxies.".into(),
                icon: "📱".into(),
                tags: vec!["android".into(), "emulator".into(), "mobile".into(), "adb".into()],
                runtime: "python".into(),
            },
            TemplateInfo {
                id: "ldplayer_title_grabber".into(),
                name: "LDPlayer Title Grabber".into(),
                description: "Automate LDPlayer emulator: launch instance, open URL in browser, extract page title. Uses ldconsole + ADB.".into(),
                icon: "🎮".into(),
                tags: vec!["ldplayer".into(), "emulator".into(), "scraping".into(), "title".into(), "adb".into()],
                runtime: "python".into(),
            },
        ]
    }

    /// Parse a template from its string ID.
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "shield_scraper" => Ok(ActorTemplate::ShieldScraper),
            "shield_crawler" => Ok(ActorTemplate::ShieldCrawler),
            "shield_screenshot" => Ok(ActorTemplate::ShieldScreenshot),
            "shield_form_filler" => Ok(ActorTemplate::ShieldFormFiller),
            "shield_social_bot" => Ok(ActorTemplate::ShieldSocialBot),
            "emulator_bot" => Ok(ActorTemplate::EmulatorBot),
            "ldplayer_title_grabber" => Ok(ActorTemplate::LdPlayerTitleGrabber),
            _ => anyhow::bail!("Unknown actor template: '{}'", s),
        }
    }

    /// Scaffold a new actor project from this template.
    pub fn scaffold(&self, dir: &Path, actor_name: &str) -> Result<()> {
        std::fs::create_dir_all(dir).context("Failed to create actor directory")?;

        let actor_id = slugify(actor_name);
        let manifest = self.build_manifest(&actor_id, actor_name);

        // Write nde_actor.json
        manifest.save(dir)?;

        // Create source directory and entry point
        let src_dir = dir.join("src");
        std::fs::create_dir_all(&src_dir)?;

        let main_py = self.generate_main_py(&actor_id);
        std::fs::write(src_dir.join("main.py"), main_py)?;

        // Create __init__.py for Python module
        std::fs::write(src_dir.join("__init__.py"), "")?;

        // Write requirements.txt
        let reqs = self.generate_requirements();
        std::fs::write(dir.join("requirements.txt"), reqs)?;

        // Write README.md
        let readme = self.generate_readme(actor_name, &manifest.description);
        std::fs::write(dir.join("README.md"), readme)?;

        // Generate Apify-compatible files
        self.generate_apify_compat(dir, &manifest)?;

        tracing::info!(
            "Scaffolded actor '{}' from template {:?} at {}",
            actor_id,
            self,
            dir.display()
        );

        Ok(())
    }

    /// Generate Apify-compatible directory structure.
    fn generate_apify_compat(&self, dir: &Path, manifest: &ActorManifest) -> Result<()> {
        let apify_dir = dir.join(".actor");
        std::fs::create_dir_all(&apify_dir)?;

        // .actor/actor.json
        let actor_json = manifest.to_apify_actor_json();
        std::fs::write(
            apify_dir.join("actor.json"),
            serde_json::to_string_pretty(&actor_json)?,
        )?;

        // .actor/input_schema.json
        let input_schema = manifest.to_apify_input_schema();
        std::fs::write(
            apify_dir.join("input_schema.json"),
            serde_json::to_string_pretty(&input_schema)?,
        )?;

        // Dockerfile
        let dockerfile = manifest.to_dockerfile();
        std::fs::write(dir.join("Dockerfile"), dockerfile)?;

        Ok(())
    }

    /// Build the manifest for this template.
    fn build_manifest(&self, actor_id: &str, actor_name: &str) -> ActorManifest {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        match self {
            ActorTemplate::ShieldScraper => ActorManifest {
                id: actor_id.to_string(),
                name: actor_name.to_string(),
                version: "1.0.0".to_string(),
                description: "Scrapes data from websites using Shield anti-detect browser"
                    .to_string(),
                author: None,
                tags: vec!["scraping".into(), "anti-detect".into()],
                icon: Some("🕷️".into()),
                input_schema: scraper_input_schema(),
                runtime: ActorRuntime::Python {
                    version: "3.11".to_string(),
                    pip_deps: vec!["playwright".into(), "nde-actor-sdk".into()],
                    entry: "src/main.py".to_string(),
                },
                browser: BrowserConfig {
                    engine: None,
                    headless: true,
                    proxy_from_profile: true,
                    profile_id: None,
                    pages: 1,
                },
                output: OutputConfig::default(),
                apify: Some(ApifyConfig::default()),
                created_at: now,
            },
            ActorTemplate::ShieldCrawler => ActorManifest {
                id: actor_id.to_string(),
                name: actor_name.to_string(),
                version: "1.0.0".to_string(),
                description: "Crawls multi-page websites with URL queue and depth control"
                    .to_string(),
                author: None,
                tags: vec!["crawling".into(), "anti-detect".into()],
                icon: Some("🔗".into()),
                input_schema: crawler_input_schema(),
                runtime: ActorRuntime::Python {
                    version: "3.11".to_string(),
                    pip_deps: vec!["playwright".into(), "nde-actor-sdk".into()],
                    entry: "src/main.py".to_string(),
                },
                browser: BrowserConfig {
                    engine: None,
                    headless: true,
                    proxy_from_profile: true,
                    profile_id: None,
                    pages: 3,
                },
                output: OutputConfig::default(),
                apify: Some(ApifyConfig::default()),
                created_at: now,
            },
            ActorTemplate::ShieldScreenshot => ActorManifest {
                id: actor_id.to_string(),
                name: actor_name.to_string(),
                version: "1.0.0".to_string(),
                description: "Takes screenshots of URLs with Shield anti-detect browser"
                    .to_string(),
                author: None,
                tags: vec!["screenshot".into(), "visual".into()],
                icon: Some("📸".into()),
                input_schema: screenshot_input_schema(),
                runtime: ActorRuntime::Python {
                    version: "3.11".to_string(),
                    pip_deps: vec!["playwright".into(), "nde-actor-sdk".into()],
                    entry: "src/main.py".to_string(),
                },
                browser: BrowserConfig::default(),
                output: OutputConfig {
                    format: OutputFormat::Json,
                    max_items: 0,
                },
                apify: Some(ApifyConfig::default()),
                created_at: now,
            },
            ActorTemplate::ShieldFormFiller => ActorManifest {
                id: actor_id.to_string(),
                name: actor_name.to_string(),
                version: "1.0.0".to_string(),
                description: "Automates form filling with persistent Shield profiles".to_string(),
                author: None,
                tags: vec!["forms".into(), "automation".into()],
                icon: Some("📝".into()),
                input_schema: form_filler_input_schema(),
                runtime: ActorRuntime::Python {
                    version: "3.11".to_string(),
                    pip_deps: vec!["playwright".into(), "nde-actor-sdk".into()],
                    entry: "src/main.py".to_string(),
                },
                browser: BrowserConfig {
                    engine: None,
                    headless: false, // Form filling often needs visible browser
                    proxy_from_profile: true,
                    profile_id: None,
                    pages: 1,
                },
                output: OutputConfig::default(),
                apify: Some(ApifyConfig::default()),
                created_at: now,
            },
            ActorTemplate::ShieldSocialBot => ActorManifest {
                id: actor_id.to_string(),
                name: actor_name.to_string(),
                version: "1.0.0".to_string(),
                description: "Social media automation with anti-detect fingerprints".to_string(),
                author: None,
                tags: vec!["social".into(), "anti-detect".into(), "bot".into()],
                icon: Some("🤖".into()),
                input_schema: social_bot_input_schema(),
                runtime: ActorRuntime::Python {
                    version: "3.11".to_string(),
                    pip_deps: vec!["playwright".into(), "nde-actor-sdk".into()],
                    entry: "src/main.py".to_string(),
                },
                browser: BrowserConfig {
                    engine: None,
                    headless: false,
                    proxy_from_profile: true,
                    profile_id: None,
                    pages: 1,
                },
                output: OutputConfig::default(),
                apify: Some(ApifyConfig::default()),
                created_at: now,
            },
            ActorTemplate::EmulatorBot => ActorManifest {
                id: actor_id.to_string(),
                name: actor_name.to_string(),
                version: "1.0.0".to_string(),
                description: "Android emulator automation via ADB".to_string(),
                author: None,
                tags: vec!["android".into(), "emulator".into(), "mobile".into()],
                icon: Some("📱".into()),
                input_schema: emulator_input_schema(),
                runtime: ActorRuntime::Python {
                    version: "3.11".to_string(),
                    pip_deps: vec!["nde-actor-sdk".into()],
                    entry: "src/main.py".to_string(),
                },
                browser: BrowserConfig::default(),
                output: OutputConfig::default(),
                apify: None, // Emulator actors don't deploy to Apify
                created_at: now,
            },
            ActorTemplate::LdPlayerTitleGrabber => ActorManifest {
                id: actor_id.to_string(),
                name: actor_name.to_string(),
                version: "1.0.0".to_string(),
                description: "Launch LDPlayer instance, open URL, grab website title via ADB"
                    .to_string(),
                author: None,
                tags: vec![
                    "ldplayer".into(),
                    "emulator".into(),
                    "scraping".into(),
                    "title".into(),
                ],
                icon: Some("🎮".into()),
                input_schema: ldplayer_title_input_schema(),
                runtime: ActorRuntime::Python {
                    version: "3.11".to_string(),
                    pip_deps: vec!["nde-actor-sdk".into()],
                    entry: "src/main.py".to_string(),
                },
                browser: BrowserConfig::default(),
                output: OutputConfig::default(),
                apify: None,
                created_at: now,
            },
        }
    }

    /// Generate the main.py source code for this template.
    fn generate_main_py(&self, _actor_id: &str) -> String {
        match self {
            ActorTemplate::ShieldScraper => SCRAPER_MAIN.to_string(),
            ActorTemplate::ShieldCrawler => CRAWLER_MAIN.to_string(),
            ActorTemplate::ShieldScreenshot => SCREENSHOT_MAIN.to_string(),
            ActorTemplate::ShieldFormFiller => FORM_FILLER_MAIN.to_string(),
            ActorTemplate::ShieldSocialBot => SOCIAL_BOT_MAIN.to_string(),
            ActorTemplate::EmulatorBot => EMULATOR_MAIN.to_string(),
            ActorTemplate::LdPlayerTitleGrabber => LDPLAYER_TITLE_MAIN.to_string(),
        }
    }

    /// Generate requirements.txt for this template.
    fn generate_requirements(&self) -> String {
        match self {
            ActorTemplate::EmulatorBot | ActorTemplate::LdPlayerTitleGrabber => {
                "nde-actor-sdk\n".to_string()
            }
            _ => "playwright\nnde-actor-sdk\n".to_string(),
        }
    }

    /// Generate README.md.
    fn generate_readme(&self, name: &str, description: &str) -> String {
        format!(
            r#"# {name}

{description}

## Running Locally (NDE-OS)

This actor runs on NDE-OS using Shield Browser for anti-detect browsing.

```bash
# Via NDE-OS API
curl -X POST http://localhost:8080/api/actors/{slug}/run \
  -H "Content-Type: application/json" \
  -d '{{"startUrls": [{{"url": "https://example.com"}}]}}'
```

## Running on Apify

This actor is Apify-compatible. Deploy with:

```bash
cd {slug}
apify push
```

## Input Schema

See `nde_actor.json` or `.actor/input_schema.json` for full input documentation.

## Output

Results are stored in the dataset (JSONL format locally, Apify Dataset in cloud).
"#,
            name = name,
            description = description,
            slug = slugify(name),
        )
    }
}

// ─── Input Schema Builders ─────────────────────────────────────────

fn scraper_input_schema() -> InputSchema {
    let mut props = HashMap::new();
    props.insert(
        "startUrls".into(),
        InputProperty {
            title: "Start URLs".into(),
            property_type: PropertyType::Array,
            description: Some("List of URLs to scrape".into()),
            default: None,
            editor: Some("requestListSources".into()),
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: Some(serde_json::json!([{"url": "https://example.com"}])),
        },
    );
    props.insert(
        "maxPages".into(),
        InputProperty {
            title: "Max Pages".into(),
            property_type: PropertyType::Integer,
            description: Some("Maximum number of pages to scrape".into()),
            default: Some(serde_json::json!(10)),
            editor: None,
            enum_values: None,
            minimum: Some(1.0),
            maximum: Some(10000.0),
            prefill: None,
        },
    );
    props.insert(
        "waitForSelector".into(),
        InputProperty {
            title: "Wait for Selector".into(),
            property_type: PropertyType::String,
            description: Some("CSS selector to wait for before extracting data".into()),
            default: Some(serde_json::json!("body")),
            editor: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: None,
        },
    );

    InputSchema {
        title: "Shield Scraper Input".into(),
        description: Some("Configuration for Shield anti-detect web scraper".into()),
        schema_type: "object".into(),
        schema_version: 1,
        properties: props,
        required: vec!["startUrls".into()],
    }
}

fn crawler_input_schema() -> InputSchema {
    let mut props = HashMap::new();
    props.insert(
        "startUrls".into(),
        InputProperty {
            title: "Start URLs".into(),
            property_type: PropertyType::Array,
            description: Some("Seed URLs to begin crawling from".into()),
            default: None,
            editor: Some("requestListSources".into()),
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: Some(serde_json::json!([{"url": "https://example.com"}])),
        },
    );
    props.insert(
        "maxDepth".into(),
        InputProperty {
            title: "Max Crawl Depth".into(),
            property_type: PropertyType::Integer,
            description: Some("Maximum link depth to follow".into()),
            default: Some(serde_json::json!(3)),
            editor: None,
            enum_values: None,
            minimum: Some(1.0),
            maximum: Some(20.0),
            prefill: None,
        },
    );
    props.insert(
        "maxPages".into(),
        InputProperty {
            title: "Max Pages".into(),
            property_type: PropertyType::Integer,
            description: Some("Maximum total pages to crawl".into()),
            default: Some(serde_json::json!(100)),
            editor: None,
            enum_values: None,
            minimum: Some(1.0),
            maximum: Some(100000.0),
            prefill: None,
        },
    );
    props.insert(
        "sameDomainOnly".into(),
        InputProperty {
            title: "Same Domain Only".into(),
            property_type: PropertyType::Boolean,
            description: Some("Only follow links on the same domain".into()),
            default: Some(serde_json::json!(true)),
            editor: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: None,
        },
    );

    InputSchema {
        title: "Shield Crawler Input".into(),
        description: Some("Configuration for Shield anti-detect web crawler".into()),
        schema_type: "object".into(),
        schema_version: 1,
        properties: props,
        required: vec!["startUrls".into()],
    }
}

fn screenshot_input_schema() -> InputSchema {
    let mut props = HashMap::new();
    props.insert(
        "urls".into(),
        InputProperty {
            title: "URLs".into(),
            property_type: PropertyType::Array,
            description: Some("URLs to screenshot".into()),
            default: None,
            editor: Some("requestListSources".into()),
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: Some(serde_json::json!([{"url": "https://example.com"}])),
        },
    );
    props.insert(
        "viewportWidth".into(),
        InputProperty {
            title: "Viewport Width".into(),
            property_type: PropertyType::Integer,
            description: Some("Browser viewport width in pixels".into()),
            default: Some(serde_json::json!(1920)),
            editor: None,
            enum_values: None,
            minimum: Some(320.0),
            maximum: Some(3840.0),
            prefill: None,
        },
    );
    props.insert(
        "viewportHeight".into(),
        InputProperty {
            title: "Viewport Height".into(),
            property_type: PropertyType::Integer,
            description: Some("Browser viewport height in pixels".into()),
            default: Some(serde_json::json!(1080)),
            editor: None,
            enum_values: None,
            minimum: Some(240.0),
            maximum: Some(2160.0),
            prefill: None,
        },
    );
    props.insert(
        "fullPage".into(),
        InputProperty {
            title: "Full Page".into(),
            property_type: PropertyType::Boolean,
            description: Some("Capture full scrollable page instead of just viewport".into()),
            default: Some(serde_json::json!(false)),
            editor: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: None,
        },
    );

    InputSchema {
        title: "Shield Screenshot Input".into(),
        description: Some("Configuration for Shield screenshot actor".into()),
        schema_type: "object".into(),
        schema_version: 1,
        properties: props,
        required: vec!["urls".into()],
    }
}

fn form_filler_input_schema() -> InputSchema {
    let mut props = HashMap::new();
    props.insert(
        "url".into(),
        InputProperty {
            title: "Form URL".into(),
            property_type: PropertyType::String,
            description: Some("URL of the page containing the form".into()),
            default: None,
            editor: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: None,
        },
    );
    props.insert(
        "fields".into(),
        InputProperty {
            title: "Form Fields".into(),
            property_type: PropertyType::Object,
            description: Some("Map of CSS selector → value to fill".into()),
            default: None,
            editor: Some("json".into()),
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: Some(serde_json::json!({"#email": "test@example.com", "#name": "John Doe"})),
        },
    );
    props.insert(
        "submitSelector".into(),
        InputProperty {
            title: "Submit Button Selector".into(),
            property_type: PropertyType::String,
            description: Some("CSS selector for the submit button".into()),
            default: Some(serde_json::json!("button[type='submit']")),
            editor: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: None,
        },
    );

    InputSchema {
        title: "Shield Form Filler Input".into(),
        description: Some("Configuration for Shield form automation".into()),
        schema_type: "object".into(),
        schema_version: 1,
        properties: props,
        required: vec!["url".into(), "fields".into()],
    }
}

fn social_bot_input_schema() -> InputSchema {
    let mut props = HashMap::new();
    props.insert(
        "platform".into(),
        InputProperty {
            title: "Platform".into(),
            property_type: PropertyType::String,
            description: Some("Social media platform".into()),
            default: None,
            editor: None,
            enum_values: Some(vec![
                "twitter".into(),
                "instagram".into(),
                "facebook".into(),
                "linkedin".into(),
                "tiktok".into(),
            ]),
            minimum: None,
            maximum: None,
            prefill: None,
        },
    );
    props.insert(
        "action".into(),
        InputProperty {
            title: "Action".into(),
            property_type: PropertyType::String,
            description: Some("What action to perform".into()),
            default: None,
            editor: None,
            enum_values: Some(vec![
                "scrape_profile".into(),
                "scrape_feed".into(),
                "scrape_search".into(),
            ]),
            minimum: None,
            maximum: None,
            prefill: None,
        },
    );
    props.insert(
        "target".into(),
        InputProperty {
            title: "Target".into(),
            property_type: PropertyType::String,
            description: Some("Username, URL, or search query".into()),
            default: None,
            editor: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: None,
        },
    );
    props.insert(
        "maxResults".into(),
        InputProperty {
            title: "Max Results".into(),
            property_type: PropertyType::Integer,
            description: Some("Maximum items to collect".into()),
            default: Some(serde_json::json!(50)),
            editor: None,
            enum_values: None,
            minimum: Some(1.0),
            maximum: Some(10000.0),
            prefill: None,
        },
    );

    InputSchema {
        title: "Shield Social Bot Input".into(),
        description: Some("Configuration for social media automation".into()),
        schema_type: "object".into(),
        schema_version: 1,
        properties: props,
        required: vec!["platform".into(), "action".into(), "target".into()],
    }
}

fn emulator_input_schema() -> InputSchema {
    let mut props = HashMap::new();
    props.insert(
        "deviceSerial".into(),
        InputProperty {
            title: "Device Serial".into(),
            property_type: PropertyType::String,
            description: Some("ADB device serial (e.g. emulator-5554, 127.0.0.1:5555)".into()),
            default: Some(serde_json::json!("emulator-5554")),
            editor: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: None,
        },
    );
    props.insert(
        "action".into(),
        InputProperty {
            title: "Action".into(),
            property_type: PropertyType::String,
            description: Some("Action to perform on the emulator".into()),
            default: None,
            editor: None,
            enum_values: Some(vec![
                "open_url".into(),
                "screenshot".into(),
                "install_apk".into(),
                "run_command".into(),
            ]),
            minimum: None,
            maximum: None,
            prefill: None,
        },
    );
    props.insert(
        "url".into(),
        InputProperty {
            title: "URL".into(),
            property_type: PropertyType::String,
            description: Some("URL to open (for open_url action)".into()),
            default: None,
            editor: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: None,
        },
    );
    props.insert(
        "command".into(),
        InputProperty {
            title: "Shell Command".into(),
            property_type: PropertyType::String,
            description: Some("ADB shell command to execute (for run_command action)".into()),
            default: None,
            editor: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: None,
        },
    );

    InputSchema {
        title: "Emulator Bot Input".into(),
        description: Some("Configuration for Android emulator automation".into()),
        schema_type: "object".into(),
        schema_version: 1,
        properties: props,
        required: vec!["deviceSerial".into(), "action".into()],
    }
}

fn ldplayer_title_input_schema() -> InputSchema {
    let mut props = HashMap::new();
    props.insert(
        "instanceName".into(),
        InputProperty {
            title: "LDPlayer Instance Name".into(),
            property_type: PropertyType::String,
            description: Some(
                "Name of the LDPlayer instance to use. If not running, the actor will launch it."
                    .into(),
            ),
            default: Some(serde_json::json!("LDPlayer")),
            editor: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: None,
        },
    );
    props.insert(
        "urls".into(),
        InputProperty {
            title: "URLs".into(),
            property_type: PropertyType::Array,
            description: Some("List of URLs to open and grab the page title from".into()),
            default: None,
            editor: Some("requestListSources".into()),
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: Some(
                serde_json::json!([{"url": "https://example.com"}, {"url": "https://github.com"}]),
            ),
        },
    );
    props.insert(
        "ldconsolePath".into(),
        InputProperty {
            title: "ldconsole Path (optional)".into(),
            property_type: PropertyType::String,
            description: Some(
                "Full path to ldconsole.exe. Leave empty for auto-detection.".into(),
            ),
            default: Some(serde_json::json!("")),
            editor: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            prefill: None,
        },
    );
    props.insert(
        "adbPort".into(),
        InputProperty {
            title: "ADB Port".into(),
            property_type: PropertyType::Integer,
            description: Some(
                "ADB port for the LDPlayer instance (default: 5555 for index 0, 5557 for index 1, etc)"
                    .into(),
            ),
            default: Some(serde_json::json!(5555)),
            editor: None,
            enum_values: None,
            minimum: Some(5555.0),
            maximum: Some(5599.0),
            prefill: None,
        },
    );
    props.insert(
        "waitSeconds".into(),
        InputProperty {
            title: "Wait Seconds".into(),
            property_type: PropertyType::Integer,
            description: Some("Seconds to wait after opening URL for page to load".into()),
            default: Some(serde_json::json!(5)),
            editor: None,
            enum_values: None,
            minimum: Some(1.0),
            maximum: Some(60.0),
            prefill: None,
        },
    );

    InputSchema {
        title: "LDPlayer Title Grabber Input".into(),
        description: Some(
            "Configuration for LDPlayer website title extraction actor".into(),
        ),
        schema_type: "object".into(),
        schema_version: 1,
        properties: props,
        required: vec!["urls".into()],
    }
}

// ─── Template Source Code ──────────────────────────────────────────

const SCRAPER_MAIN: &str = r#"""
NDE-OS Shield Scraper Actor

Extracts data from websites using Shield anti-detect browser.
Runs locally on NDE-OS (via Shield CDP) or on Apify (via Playwright).
"""
import asyncio
import json
import os
import sys

# Detect runtime and import accordingly
if os.environ.get("NDE_ACTOR"):
    # Running on NDE-OS — use local SDK
    from nde_actor_sdk import Actor
else:
    # Running on Apify — use Apify SDK
    try:
        from apify import Actor
    except ImportError:
        from nde_actor_sdk import Actor


async def main():
    await Actor.init()

    input_data = await Actor.get_input()
    start_urls = input_data.get("startUrls", [])
    max_pages = input_data.get("maxPages", 10)
    wait_selector = input_data.get("waitForSelector", "body")

    browser = await Actor.get_browser()
    page = await browser.new_page()

    for i, url_obj in enumerate(start_urls[:max_pages]):
        url = url_obj["url"] if isinstance(url_obj, dict) else url_obj

        try:
            await page.goto(url, wait_until="domcontentloaded", timeout=30000)
            await page.wait_for_selector(wait_selector, timeout=10000)

            title = await page.title()
            content = await page.content()

            await Actor.push_data([{
                "url": url,
                "title": title,
                "htmlLength": len(content),
                "timestamp": __import__("datetime").datetime.utcnow().isoformat(),
            }])

            print(f"[{i+1}/{len(start_urls)}] Scraped: {url} — {title}")

        except Exception as e:
            print(f"[{i+1}/{len(start_urls)}] Failed: {url} — {e}", file=sys.stderr)
            await Actor.push_data([{
                "url": url,
                "error": str(e),
            }])

    await browser.close()
    await Actor.exit()


if __name__ == "__main__":
    asyncio.run(main())
"#;

const CRAWLER_MAIN: &str = r#"""
NDE-OS Shield Crawler Actor

Crawls multi-page websites with URL queue and depth control.
"""
import asyncio
import os
import sys
from urllib.parse import urljoin, urlparse

if os.environ.get("NDE_ACTOR"):
    from nde_actor_sdk import Actor
else:
    try:
        from apify import Actor
    except ImportError:
        from nde_actor_sdk import Actor


async def main():
    await Actor.init()

    input_data = await Actor.get_input()
    start_urls = input_data.get("startUrls", [])
    max_depth = input_data.get("maxDepth", 3)
    max_pages = input_data.get("maxPages", 100)
    same_domain = input_data.get("sameDomainOnly", True)

    browser = await Actor.get_browser()
    page = await browser.new_page()

    visited = set()
    queue = []

    for url_obj in start_urls:
        url = url_obj["url"] if isinstance(url_obj, dict) else url_obj
        queue.append((url, 0))

    pages_crawled = 0

    while queue and pages_crawled < max_pages:
        url, depth = queue.pop(0)

        if url in visited:
            continue
        visited.add(url)

        try:
            await page.goto(url, wait_until="domcontentloaded", timeout=30000)
            title = await page.title()

            await Actor.push_data([{
                "url": url,
                "title": title,
                "depth": depth,
            }])

            pages_crawled += 1
            print(f"[{pages_crawled}/{max_pages}] Crawled (depth={depth}): {url}")

            # Extract links if not at max depth
            if depth < max_depth:
                links = await page.eval_on_selector_all(
                    "a[href]",
                    "els => els.map(e => e.href)"
                )
                base_domain = urlparse(url).netloc

                for link in links:
                    abs_url = urljoin(url, link)
                    if abs_url not in visited:
                        if not same_domain or urlparse(abs_url).netloc == base_domain:
                            queue.append((abs_url, depth + 1))

        except Exception as e:
            print(f"Failed: {url} — {e}", file=sys.stderr)

    await browser.close()
    await Actor.exit()


if __name__ == "__main__":
    asyncio.run(main())
"#;

const SCREENSHOT_MAIN: &str = r#"""
NDE-OS Shield Screenshot Actor

Takes screenshots of URLs with custom viewport settings.
"""
import asyncio
import base64
import os
import sys

if os.environ.get("NDE_ACTOR"):
    from nde_actor_sdk import Actor
else:
    try:
        from apify import Actor
    except ImportError:
        from nde_actor_sdk import Actor


async def main():
    await Actor.init()

    input_data = await Actor.get_input()
    urls = input_data.get("urls", [])
    width = input_data.get("viewportWidth", 1920)
    height = input_data.get("viewportHeight", 1080)
    full_page = input_data.get("fullPage", False)

    browser = await Actor.get_browser()
    page = await browser.new_page()
    await page.set_viewport_size({"width": width, "height": height})

    for i, url_obj in enumerate(urls):
        url = url_obj["url"] if isinstance(url_obj, dict) else url_obj

        try:
            await page.goto(url, wait_until="networkidle", timeout=30000)
            screenshot_bytes = await page.screenshot(full_page=full_page)
            screenshot_b64 = base64.b64encode(screenshot_bytes).decode()

            # Store screenshot in KV store
            key = f"screenshot_{i}"
            await Actor.set_value(key, screenshot_bytes, content_type="image/png")

            await Actor.push_data([{
                "url": url,
                "screenshotKey": key,
                "width": width,
                "height": height,
                "fullPage": full_page,
                "sizeBytes": len(screenshot_bytes),
            }])

            print(f"[{i+1}/{len(urls)}] Screenshot: {url} ({len(screenshot_bytes)} bytes)")

        except Exception as e:
            print(f"[{i+1}/{len(urls)}] Failed: {url} — {e}", file=sys.stderr)

    await browser.close()
    await Actor.exit()


if __name__ == "__main__":
    asyncio.run(main())
"#;

const FORM_FILLER_MAIN: &str = r#"""
NDE-OS Shield Form Filler Actor

Automates form filling and submission with Shield profiles.
"""
import asyncio
import os
import sys

if os.environ.get("NDE_ACTOR"):
    from nde_actor_sdk import Actor
else:
    try:
        from apify import Actor
    except ImportError:
        from nde_actor_sdk import Actor


async def main():
    await Actor.init()

    input_data = await Actor.get_input()
    url = input_data["url"]
    fields = input_data["fields"]
    submit_selector = input_data.get("submitSelector", "button[type='submit']")

    browser = await Actor.get_browser()
    page = await browser.new_page()

    await page.goto(url, wait_until="domcontentloaded", timeout=30000)
    print(f"Navigated to: {url}")

    # Fill form fields
    for selector, value in fields.items():
        try:
            await page.fill(selector, str(value))
            print(f"  Filled {selector} = {value}")
        except Exception as e:
            print(f"  Failed to fill {selector}: {e}", file=sys.stderr)

    # Submit the form
    try:
        await page.click(submit_selector)
        await page.wait_for_load_state("networkidle", timeout=15000)
        print("Form submitted successfully")

        result_url = page.url
        result_title = await page.title()

        await Actor.push_data([{
            "formUrl": url,
            "resultUrl": result_url,
            "resultTitle": result_title,
            "fieldsCount": len(fields),
            "success": True,
        }])
    except Exception as e:
        print(f"Form submission failed: {e}", file=sys.stderr)
        await Actor.push_data([{
            "formUrl": url,
            "error": str(e),
            "success": False,
        }])

    await browser.close()
    await Actor.exit()


if __name__ == "__main__":
    asyncio.run(main())
"#;

const SOCIAL_BOT_MAIN: &str = r#"""
NDE-OS Shield Social Bot Actor

Social media automation with anti-detect fingerprints.
"""
import asyncio
import os
import sys

if os.environ.get("NDE_ACTOR"):
    from nde_actor_sdk import Actor
else:
    try:
        from apify import Actor
    except ImportError:
        from nde_actor_sdk import Actor


async def main():
    await Actor.init()

    input_data = await Actor.get_input()
    platform = input_data["platform"]
    action = input_data["action"]
    target = input_data["target"]
    max_results = input_data.get("maxResults", 50)

    browser = await Actor.get_browser()
    page = await browser.new_page()

    print(f"Platform: {platform}, Action: {action}, Target: {target}")

    # Platform-specific logic (extend as needed)
    if action == "scrape_profile":
        await scrape_profile(page, platform, target)
    elif action == "scrape_feed":
        await scrape_feed(page, platform, target, max_results)
    elif action == "scrape_search":
        await scrape_search(page, platform, target, max_results)

    await browser.close()
    await Actor.exit()


async def scrape_profile(page, platform, target):
    """Scrape a user profile page."""
    urls = {
        "twitter": f"https://twitter.com/{target}",
        "instagram": f"https://instagram.com/{target}",
        "facebook": f"https://facebook.com/{target}",
        "linkedin": f"https://linkedin.com/in/{target}",
        "tiktok": f"https://tiktok.com/@{target}",
    }

    url = urls.get(platform, f"https://{platform}.com/{target}")
    await page.goto(url, wait_until="domcontentloaded", timeout=30000)

    title = await page.title()
    await Actor.push_data([{
        "platform": platform,
        "action": "scrape_profile",
        "target": target,
        "url": url,
        "pageTitle": title,
    }])


async def scrape_feed(page, platform, target, max_results):
    """Scrape a feed or timeline."""
    print(f"TODO: Implement {platform} feed scraping for {target}")
    await Actor.push_data([{
        "platform": platform,
        "action": "scrape_feed",
        "target": target,
        "status": "template — implement platform-specific logic",
    }])


async def scrape_search(page, platform, target, max_results):
    """Scrape search results."""
    print(f"TODO: Implement {platform} search scraping for '{target}'")
    await Actor.push_data([{
        "platform": platform,
        "action": "scrape_search",
        "query": target,
        "status": "template — implement platform-specific logic",
    }])


if __name__ == "__main__":
    asyncio.run(main())
"#;

const EMULATOR_MAIN: &str = r#"""
NDE-OS Emulator Bot Actor

Android emulator automation via ADB.
This actor runs only on NDE-OS (no Apify deployment).
"""
import asyncio
import os
import sys
import subprocess

if os.environ.get("NDE_ACTOR"):
    from nde_actor_sdk import Actor
else:
    from nde_actor_sdk import Actor


def adb(*args, serial=None):
    """Run an ADB command and return stdout."""
    cmd = ["adb"]
    if serial:
        cmd.extend(["-s", serial])
    cmd.extend(args)
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    if result.returncode != 0:
        raise RuntimeError(f"ADB command failed: {result.stderr.strip()}")
    return result.stdout.strip()


async def main():
    await Actor.init()

    input_data = await Actor.get_input()
    serial = input_data["deviceSerial"]
    action = input_data["action"]

    print(f"Device: {serial}, Action: {action}")

    if action == "open_url":
        url = input_data.get("url", "https://example.com")
        adb("shell", "am", "start", "-a", "android.intent.action.VIEW", "-d", url, serial=serial)
        await Actor.push_data([{"action": "open_url", "url": url, "device": serial, "success": True}])

    elif action == "screenshot":
        device_path = "/sdcard/nde_screenshot.png"
        adb("shell", "screencap", "-p", device_path, serial=serial)

        import tempfile
        with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as f:
            local_path = f.name

        adb("pull", device_path, local_path, serial=serial)
        adb("shell", "rm", device_path, serial=serial)

        with open(local_path, "rb") as f:
            screenshot_bytes = f.read()

        await Actor.set_value("screenshot", screenshot_bytes, content_type="image/png")
        os.unlink(local_path)

        await Actor.push_data([{"action": "screenshot", "device": serial, "sizeBytes": len(screenshot_bytes)}])

    elif action == "run_command":
        command = input_data.get("command", "getprop ro.build.version.release")
        output = adb("shell", command, serial=serial)
        await Actor.push_data([{"action": "run_command", "command": command, "output": output, "device": serial}])

    elif action == "install_apk":
        apk_path = input_data.get("apkPath", "")
        if apk_path:
            adb("install", "-r", apk_path, serial=serial)
            await Actor.push_data([{"action": "install_apk", "apk": apk_path, "device": serial, "success": True}])

    await Actor.exit()


if __name__ == "__main__":
    asyncio.run(main())
"#;

const LDPLAYER_TITLE_MAIN: &str = r#"""
NDE-OS LDPlayer Title Grabber Actor

Automates LDPlayer to open URLs and extract website page titles.
Uses ldconsole for instance lifecycle + ADB for browser interaction.
"""

import asyncio
import os
import re
import shutil
import subprocess
import time


try:
    from nde_actor_sdk import Actor
except ImportError:
    # Minimal fallback for standalone execution
    class Actor:
        @staticmethod
        async def init(): pass
        @staticmethod
        async def get_input():
            return {
                "urls": [{"url": "https://example.com"}],
                "instanceName": "LDPlayer",
                "adbPort": 5555,
                "waitSeconds": 5,
            }
        @staticmethod
        async def push_data(data): print(f"[OUTPUT] {data}")
        @staticmethod
        async def exit(): pass


# ─── LDPlayer Detection ───────────────────────────────────────

LDPLAYER_DIRS = ["LDPlayer", "LDPlayer9", "LDPlayer4", "LDPlayer5"]

def find_ldconsole(custom_path=""):
    """Auto-detect ldconsole.exe location."""
    if custom_path and os.path.isfile(custom_path):
        return custom_path

    # Check PATH
    found = shutil.which("ldconsole.exe") or shutil.which("ldconsole")
    if found:
        return found

    # Check common Windows paths
    for env_key in ["ProgramFiles", "ProgramFiles(x86)"]:
        base = os.environ.get(env_key, "")
        if base:
            for d in LDPLAYER_DIRS:
                candidate = os.path.join(base, d, "ldconsole.exe")
                if os.path.isfile(candidate):
                    return candidate

    for drive in ["C:", "D:", "E:"]:
        for d in LDPLAYER_DIRS:
            candidate = os.path.join(drive + os.sep, d, "ldconsole.exe")
            if os.path.isfile(candidate):
                return candidate

    return None


def ldconsole(ldpath, *args):
    """Run an ldconsole command and return stdout."""
    result = subprocess.run(
        [ldpath] + list(args),
        capture_output=True, text=True, timeout=60
    )
    return result.stdout.strip()


def is_instance_running(ldpath, name):
    """Check if an LDPlayer instance is running."""
    output = ldconsole(ldpath, "isrunning", "--name", name)
    return output.strip().lower() == "running"


# ─── ADB Helpers ───────────────────────────────────────────────

def adb(*args, serial=None):
    """Execute an ADB command against a specific device."""
    cmd = ["adb"]
    if serial:
        cmd.extend(["-s", serial])
    cmd.extend(args)
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    if result.returncode != 0:
        raise RuntimeError(f"ADB command failed: {result.stderr.strip()}")
    return result.stdout.strip()


def wait_for_device(serial, timeout=60):
    """Wait until the ADB device is online."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            output = adb("devices")
            if serial in output and "device" in output.split(serial)[1].split("\n")[0]:
                return True
        except Exception:
            pass
        time.sleep(2)
    return False


def get_page_title(serial):
    """Extract the current browser page title from the emulator.

    Strategy:
    1. Try `dumpsys window windows` — look for the focused window title.
    2. Try `dumpsys activity top` — look for Activity label.
    3. Fallback to the focused window class name.
    """
    try:
        # Strategy 1: dumpsys window — mCurrentFocus or mFocusedWindow
        output = adb("shell", "dumpsys", "window", "windows", serial=serial)
        # Look for the focused window title line
        for line in output.splitlines():
            if "mCurrentFocus" in line or "mFocusedWindow" in line:
                # Format: mCurrentFocus=Window{hash u0 com.android.browser/...}
                match = re.search(r"Window\{[^\s]+\s+\S+\s+(.+?)\}", line)
                if match:
                    title = match.group(1)
                    if "/" in title:
                        title = title.split("/")[0]
                    return title
    except Exception:
        pass

    try:
        # Strategy 2: dumpsys activity top
        output = adb("shell", "dumpsys", "activity", "top", serial=serial)
        for line in output.splitlines():
            if "ACTIVITY" in line:
                match = re.search(r"ACTIVITY\s+(\S+)", line)
                if match:
                    return match.group(1)
    except Exception:
        pass

    return "(unknown)"


def get_browser_url(serial):
    """Try to get the current URL from the browser.

    Attempts to read from common Android browser content providers.
    """
    try:
        # Try Chrome's tabs content provider
        output = adb(
            "shell",
            "content", "query",
            "--uri", "content://com.android.chrome.browser/bookmarks",
            "--projection", "url",
            serial=serial,
        )
        urls = re.findall(r"url=(\S+)", output)
        if urls:
            return urls[0]
    except Exception:
        pass

    try:
        # Try default browser content provider
        output = adb(
            "shell",
            "content", "query",
            "--uri", "content://browser/bookmarks",
            "--projection", "url",
            serial=serial,
        )
        urls = re.findall(r"url=(\S+)", output)
        if urls:
            return urls[0]
    except Exception:
        pass

    return None


# ─── Main Actor Logic ─────────────────────────────────────────

async def main():
    await Actor.init()

    input_data = await Actor.get_input()

    instance_name = input_data.get("instanceName", "LDPlayer")
    urls = input_data.get("urls", [{"url": "https://example.com"}])
    custom_ldconsole = input_data.get("ldconsolePath", "")
    adb_port = input_data.get("adbPort", 5555)
    wait_secs = input_data.get("waitSeconds", 5)

    serial = f"127.0.0.1:{adb_port}"
    results = []

    # ── Detect ldconsole ─────────────────────────────────────
    ldpath = find_ldconsole(custom_ldconsole)
    if not ldpath:
        print("[ERROR] ldconsole.exe not found. Install LDPlayer or provide ldconsolePath.")
        await Actor.push_data([{"error": "ldconsole.exe not found"}])
        await Actor.exit()
        return

    print(f"[INFO] Using ldconsole: {ldpath}")
    print(f"[INFO] Instance: {instance_name}, ADB serial: {serial}")

    # ── Launch instance if not running ───────────────────────
    was_running = is_instance_running(ldpath, instance_name)
    if not was_running:
        print(f"[INFO] Launching LDPlayer instance '{instance_name}'...")
        ldconsole(ldpath, "launch", "--name", instance_name)
        time.sleep(10)  # Wait for emulator boot

    # ── Connect ADB ──────────────────────────────────────────
    print(f"[INFO] Connecting ADB to {serial}...")
    adb("connect", serial)
    if not wait_for_device(serial, timeout=30):
        print(f"[ERROR] Device {serial} not online after 30s")
        await Actor.push_data([{"error": f"Device {serial} not reachable"}])
        await Actor.exit()
        return

    print(f"[INFO] Device {serial} is online")

    # ── Process each URL ─────────────────────────────────────
    for entry in urls:
        url = entry if isinstance(entry, str) else entry.get("url", "")
        if not url:
            continue

        print(f"[INFO] Opening URL: {url}")

        # Open URL in default browser
        adb(
            "shell", "am", "start",
            "-a", "android.intent.action.VIEW",
            "-d", url,
            serial=serial,
        )

        # Wait for page to load
        time.sleep(wait_secs)

        # Extract title
        title = get_page_title(serial)
        browser_url = get_browser_url(serial) or url

        result = {
            "url": url,
            "title": title,
            "browserUrl": browser_url,
            "device": serial,
            "instance": instance_name,
            "success": True,
        }
        results.append(result)
        print(f"[OK] {url} → {title}")

    # ── Push results ─────────────────────────────────────────
    await Actor.push_data(results)

    print(f"\n[DONE] Grabbed titles from {len(results)} URL(s)")
    await Actor.exit()


if __name__ == "__main__":
    asyncio.run(main())
"#;

// ─── Helpers ───────────────────────────────────────────────────────

/// Convert a name to a URL-safe slug: lowercase, spaces→hyphens, strip specials.
fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' | '-' => c,
            ' ' | '_' => '-',
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_template_list() {
        let templates = ActorTemplate::all();
        assert_eq!(templates.len(), 7);
        assert!(templates.iter().any(|t| t.id == "shield_scraper"));
        assert!(templates.iter().any(|t| t.id == "emulator_bot"));
    }

    #[test]
    fn test_template_parse() {
        assert_eq!(
            ActorTemplate::from_str("shield_scraper").unwrap(),
            ActorTemplate::ShieldScraper
        );
        assert!(ActorTemplate::from_str("nonexistent").is_err());
    }

    #[test]
    fn test_scaffold_creates_files() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("my-scraper");

        ActorTemplate::ShieldScraper
            .scaffold(&dir, "My Scraper")
            .unwrap();

        // Check created files
        assert!(dir.join("nde_actor.json").exists());
        assert!(dir.join("src/main.py").exists());
        assert!(dir.join("src/__init__.py").exists());
        assert!(dir.join("requirements.txt").exists());
        assert!(dir.join("README.md").exists());
        assert!(dir.join(".actor/actor.json").exists());
        assert!(dir.join(".actor/input_schema.json").exists());
        assert!(dir.join("Dockerfile").exists());

        // Verify manifest is parseable
        let manifest = ActorManifest::load(&dir).unwrap();
        assert_eq!(manifest.id, "my-scraper");
        assert_eq!(manifest.name, "My Scraper");
        assert!(manifest.apify.is_some());
    }

    #[test]
    fn test_scaffold_all_templates() {
        let tmp = TempDir::new().unwrap();
        let templates = [
            ActorTemplate::ShieldScraper,
            ActorTemplate::ShieldCrawler,
            ActorTemplate::ShieldScreenshot,
            ActorTemplate::ShieldFormFiller,
            ActorTemplate::ShieldSocialBot,
            ActorTemplate::EmulatorBot,
            ActorTemplate::LdPlayerTitleGrabber,
        ];

        for template in &templates {
            let name = format!("{:?}", template);
            let dir = tmp.path().join(slugify(&name));
            template.scaffold(&dir, &name).unwrap();
            assert!(dir.join("nde_actor.json").exists());
            assert!(dir.join("src/main.py").exists());
        }
    }

    #[test]
    fn test_emulator_template_no_apify() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("emu-bot");
        ActorTemplate::EmulatorBot
            .scaffold(&dir, "Emu Bot")
            .unwrap();

        let manifest = ActorManifest::load(&dir).unwrap();
        assert!(manifest.apify.is_none());
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("My Cool Scraper"), "my-cool-scraper");
        assert_eq!(slugify("test_actor_123"), "test-actor-123");
        assert_eq!(slugify("Hello World!!!"), "hello-world");
        assert_eq!(slugify("  spaces  "), "spaces");
    }
}
