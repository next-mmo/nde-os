const fs = require('fs');
const content = fs.readFileSync('desktop/src/components/apps/VibeCodeStudio/ide/CodeEditor.svelte', 'utf8');
console.log("Found ONE_DARK_PRO_RULES length:", content.length);
