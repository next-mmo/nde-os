<svelte:options runes={true} />

<script lang="ts">
  import * as api from "$lib/api/backend";
  import type { SkillInfo } from "$lib/api/types";

  let skills = $state<SkillInfo[]>([]);
  let loading = $state(true);
  let searchQuery = $state("");

  $effect(() => { refresh(); });

  async function refresh() {
    loading = true;
    try { skills = await api.listSkills(); }
    catch { skills = getFallbackSkills(); }
    finally { loading = false; }
  }

  function getFallbackSkills(): SkillInfo[] {
    return [
      { name: "brainstorming", description: "Explore user intent and design decisions before planning", path: "/skills/brainstorming", triggers: ["brainstorm", "think through", "explore approaches"] },
      { name: "frontend-design", description: "Create distinctive, production-grade frontend interfaces", path: "/skills/frontend-design", triggers: ["build UI", "create component", "web design"] },
      { name: "security-sentinel", description: "Performs security audits for vulnerabilities and OWASP compliance", path: "/skills/security-sentinel", triggers: ["security review", "vulnerability check"] },
      { name: "performance-oracle", description: "Analyzes code for performance bottlenecks and scalability", path: "/skills/performance-oracle", triggers: ["performance", "optimize", "benchmark"] },
      { name: "git-history-analyzer", description: "Archaeological analysis of git history to trace code evolution", path: "/skills/git-history-analyzer", triggers: ["git history", "code evolution", "blame"] },
      { name: "compound-docs", description: "Capture solved problems as categorized documentation", path: "/skills/compound-docs", triggers: ["document", "capture solution"] },
    ];
  }

  const filtered = $derived(
    searchQuery.trim()
      ? skills.filter(s => s.name.toLowerCase().includes(searchQuery.toLowerCase()) || s.description.toLowerCase().includes(searchQuery.toLowerCase()))
      : skills
  );
</script>

<section class="skills-app">
  <div class="header">
    <div>
      <p class="eyebrow">Agent Intelligence</p>
      <h2>Skills Library</h2>
    </div>
    <div class="header-actions">
      <input class="search" type="text" placeholder="Search skills..." bind:value={searchQuery} />
      <button class="action-btn" onclick={refresh} disabled={loading}>{loading ? "..." : "↻"}</button>
    </div>
  </div>

  <p class="intro">Skills are specialized instruction sets (SKILL.md) that extend the agent's capabilities. Each skill defines triggers, workflows, and domain expertise.</p>

  {#if loading}
    <div class="loading">Loading skills...</div>
  {:else}
    <div class="stats-row">
      <div class="stat-chip">{skills.length} Skills Available</div>
      <div class="stat-chip">{skills.reduce((acc, s) => acc + s.triggers.length, 0)} Trigger Patterns</div>
    </div>

    <div class="skills-grid">
      {#each filtered as skill (skill.name)}
        <article class="skill-card">
          <div class="skill-header">
            <span class="skill-icon">📘</span>
            <div class="skill-info">
              <strong>{skill.name}</strong>
              <span class="skill-path">{skill.path}</span>
            </div>
          </div>
          <p class="skill-desc">{skill.description}</p>
          <div class="triggers">
            {#each skill.triggers as trigger}
              <span class="trigger-tag">{trigger}</span>
            {/each}
          </div>
        </article>
      {:else}
        <p class="muted">No skills match your search.</p>
      {/each}
    </div>
  {/if}
</section>

<style>
  .skills-app { height: 100%; overflow: auto; padding: 1.1rem; display: grid; gap: 1rem; align-content: start; }
  .header { display: flex; justify-content: space-between; align-items: center; gap: 1rem; flex-wrap: wrap; }
  .eyebrow { margin: 0; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.14em; color: var(--system-color-text-muted); }
  h2 { margin: 0.3rem 0 0; }
  .intro { margin: 0; font-size: 0.85rem; color: var(--system-color-text-muted); max-width: 600px; }
  .header-actions { display: flex; gap: 0.5rem; align-items: center; }
  .search {
    padding: 0.45rem 0.8rem; border-radius: 999px; border: 1px solid var(--system-color-border);
    background: var(--system-color-panel); color: var(--system-color-text); font-size: 0.82rem; width: 14rem;
  }
  .search:focus { border-color: var(--system-color-primary); outline: none; }
  .action-btn {
    border-radius: 999px; padding: 0.55rem 0.8rem; font-size: 0.82rem; cursor: pointer;
    border: 1px solid var(--system-color-border); background: var(--system-color-panel); color: var(--system-color-text);
  }
  .stats-row { display: flex; gap: 0.5rem; flex-wrap: wrap; }
  .stat-chip {
    padding: 0.35rem 0.75rem; border-radius: 999px; font-size: 0.78rem; font-weight: 600;
    background: hsla(var(--system-color-primary-hsl) / 0.08); color: var(--system-color-primary);
    border: 1px solid hsla(var(--system-color-primary-hsl) / 0.15);
  }
  .skills-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr)); gap: 0.75rem; }
  .skill-card {
    border-radius: 1rem; padding: 1rem; border: 1px solid var(--system-color-border);
    background: var(--system-color-panel); display: flex; flex-direction: column; gap: 0.5rem; transition: all 0.2s;
  }
  .skill-card:hover { border-color: hsla(var(--system-color-primary-hsl) / 0.35); transform: translateY(-1px); }
  .skill-header { display: flex; align-items: center; gap: 0.5rem; }
  .skill-icon { font-size: 1.3rem; }
  .skill-info { display: flex; flex-direction: column; }
  .skill-info strong { font-size: 0.88rem; }
  .skill-path { font-size: 0.68rem; color: var(--system-color-text-muted); font-family: ui-monospace, monospace; }
  .skill-desc { margin: 0; font-size: 0.78rem; color: var(--system-color-text-muted); line-height: 1.4; }
  .triggers { display: flex; flex-wrap: wrap; gap: 0.25rem; }
  .trigger-tag {
    font-size: 0.68rem; padding: 0.15rem 0.45rem; border-radius: 999px;
    background: hsla(40 80% 55% / 0.1); color: hsl(40 80% 60%); border: 1px solid hsla(40 80% 55% / 0.15);
  }
  .loading, .muted { color: var(--system-color-text-muted); font-size: 0.82rem; padding: 1rem 0; }
</style>
