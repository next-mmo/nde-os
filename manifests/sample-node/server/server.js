const express = require('express');
const cors = require('cors');
const path = require('path');

const app = express();
const port = process.env.PORT || 3000;

app.use(cors());
app.use(express.json());

let counter = 0;

// API routes
app.get('/api/count', (req, res) => {
  res.json({ count: counter });
});

app.post('/api/increment', (req, res) => {
  counter++;
  res.json({ count: counter });
});

app.post('/api/decrement', (req, res) => {
  counter--;
  res.json({ count: counter });
});

app.post('/api/reset', (req, res) => {
  counter = 0;
  res.json({ count: counter });
});

// Serve frontend
const distPath = path.join(__dirname, '..', 'ui', 'dist');
app.use(express.static(distPath));

// Fallback to index.html for SPA
app.get('*', (req, res) => {
  res.sendFile(path.join(distPath, 'index.html'));
});

app.listen(port, () => {
  console.log(`Node Fullstack Counter running on http://localhost:${port}`);
});
