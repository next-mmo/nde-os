"""
NDE Actor SDK — Browser abstraction.

Provides a unified browser interface that works on both:
- NDE-OS: connects to Shield Browser via Chrome DevTools Protocol (CDP)
- Apify: launches a Playwright browser instance

The `get_browser()` function auto-detects the runtime and returns
a Playwright Browser object connected to the appropriate backend.
"""
import os
from typing import Optional


async def get_browser(headless: Optional[bool] = None):
    """Get a browser instance for the current runtime.

    On NDE-OS:
        Connects to Shield Browser (Camoufox/Wayfern) via CDP.
        The CDP endpoint is passed via NDE_CDP_ENDPOINT env var,
        or defaults to ws://127.0.0.1:{NDE_CDP_PORT}/devtools/browser.

    On Apify:
        Uses Playwright to launch a new Chromium browser.

    Returns:
        playwright.async_api.Browser instance
    """
    is_nde = bool(os.environ.get("NDE_ACTOR"))

    if headless is None:
        headless = bool(os.environ.get("NDE_HEADLESS", "0") != "0")

    from playwright.async_api import async_playwright

    pw = await async_playwright().start()

    if is_nde:
        # Connect to Shield Browser via CDP
        cdp_endpoint = os.environ.get("NDE_CDP_ENDPOINT")

        if not cdp_endpoint:
            cdp_port = os.environ.get("NDE_CDP_PORT")
            if cdp_port:
                cdp_endpoint = f"http://127.0.0.1:{cdp_port}"

        if cdp_endpoint:
            print(f"[NDE Actor SDK] Connecting to Shield Browser via CDP: {cdp_endpoint}")
            browser = await pw.chromium.connect_over_cdp(cdp_endpoint)
            return browser
        else:
            # No CDP endpoint — launch a local browser as fallback
            print("[NDE Actor SDK] No CDP endpoint found, launching local Chromium")
            browser = await pw.chromium.launch(headless=headless)
            return browser
    else:
        # Apify or standalone — launch Playwright browser
        browser = await pw.chromium.launch(
            headless=headless,
            args=[
                "--no-sandbox",
                "--disable-setuid-sandbox",
                "--disable-dev-shm-usage",
            ],
        )
        return browser
