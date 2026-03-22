import { chromium } from "playwright";
(async () => {
   const b = await chromium.launch();
   const p = await b.newPage();
   await p.goto("http://localhost:5173/catalog");
   await p.waitForTimeout(2000); // wait for API calls
   
   const cards = await p.locator('div.card').all();
   for (const c of cards) {
      const text = await c.innerText();
      if (text.includes("Node.js")) {
         console.log("---- NODE.JS CARD INNER TEXT ----");
         console.log(text);
         
         const btns = await c.locator("button").all();
         console.log("Buttons:");
         for (const btn of btns) {
            console.log(await btn.innerText());
         }
      }
   }
   await b.close();
})();
