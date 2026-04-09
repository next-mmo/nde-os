// Quick validation of the wallpaper fetch + blob flow
async function testBlobFlow() {
  console.log("=== Testing full blob URL flow (simulates wallpaper.svelte.ts) ===\n");

  // 1. Build URL (same as buildWallpaperUrl)
  const seed = Math.floor(Math.random() * 1_000_000);
  const picsumUrl = `https://picsum.photos/seed/${seed}/320/240`;
  const flickrUrl = `https://loremflickr.com/320/240/nature?lock=${seed}`;

  for (const [label, url] of [["picsum (random)", picsumUrl], ["loremflickr (nature)", flickrUrl]]) {
    console.log(`--- ${label} ---`);
    console.log("  URL:", url);

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 15_000);

    try {
      const response = await fetch(url, { signal: controller.signal });
      clearTimeout(timeout);
      console.log("  Status:", response.status, response.statusText);
      console.log("  Redirected to:", response.url);

      if (!response.ok) {
        console.log("  ✗ FAILED - bad HTTP status");
        continue;
      }

      const blob = await response.blob();
      console.log("  Blob:", blob.size, "bytes |", blob.type);
      console.log("  ✓ PASS — blob created successfully");
      console.log("  In browser: URL.createObjectURL(blob) → blob:http://localhost/xxx");
      console.log("  CSS would be: url(\"blob:http://localhost/xxx\") — same-origin, no CORS\n");
    } catch (err) {
      clearTimeout(timeout);
      console.log("  ✗ FAILED:", err.message, "\n");
    }
  }

  console.log("=== All tests complete ===");
}

testBlobFlow();
