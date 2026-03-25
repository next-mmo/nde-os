---
name: research
description: Deep research on any topic using web search and browsing. Synthesizes information from multiple sources into a comprehensive report.
triggers:
  - research this
  - find out about
  - what is the best
  - compare options for
  - investigate
---

# Research Skill

## Objective

Conduct thorough research on a topic by searching the web, reading multiple sources, and synthesizing findings into a clear report.

## Steps

1. **Understand the question**: Break down what needs to be researched
2. **Search**: Use `web_search` with 2-3 different query formulations
3. **Browse**: Read the top 3-5 most relevant results using `web_browse`
4. **Cross-reference**: Compare information across sources, note agreements and conflicts
5. **Synthesize**: Combine findings into a structured report
6. **Cite sources**: Include URLs for all claims

## Tools Used

- `web_search` — find relevant pages
- `web_browse` — read full page content
- `http_fetch` — access APIs for data
- `memory_store` — save research findings for later
- `knowledge_store` — add entities/relations to knowledge graph

## Output Format

```
## Research Report: {topic}

### Summary
{2-3 sentence overview}

### Key Findings
1. {finding with source}
2. {finding with source}
3. {finding with source}

### Detailed Analysis
{in-depth discussion}

### Sources
- [Title](url) — what was learned
- [Title](url) — what was learned
```
