<svelte:options runes={true} />

<script lang="ts">
  import * as api from "$lib/api/backend";
  import type { ChannelStatus } from "$lib/api/types";

  let channels = $state<ChannelStatus[]>([]);
  let loading = $state(true);

  const TYPE_ICONS: Record<string, string> = {
    rest: "🌐", telegram: "✈️", discord: "💬", slack: "📱", web_chat: "🖥️", cli: "⌨️",
  };

  $effect(() => { refresh(); });

  async function refresh() {
    loading = true;
    try { channels = await api.listChannels(); }
    catch { channels = getFallbackChannels(); }
    finally { loading = false; }
  }

  function getFallbackChannels(): ChannelStatus[] {
    return [
      { name: "rest-api", channel_type: "rest", is_running: true, messages_received: 0, messages_sent: 0 },
      { name: "telegram-bot", channel_type: "telegram", is_running: false, messages_received: 0, messages_sent: 0 },
      { name: "discord-bot", channel_type: "discord", is_running: false, messages_received: 0, messages_sent: 0 },
      { name: "slack-bot", channel_type: "slack", is_running: false, messages_received: 0, messages_sent: 0 },
    ];
  }
</script>

<section class="channels-app">
  <div class="header">
    <div>
      <p class="eyebrow">Messaging</p>
      <h2>Channel Gateway</h2>
    </div>
    <button class="action-btn" onclick={refresh} disabled={loading}>{loading ? "Loading..." : "↻ Refresh"}</button>
  </div>

  <p class="intro">NDE-OS normalizes messages from multiple platforms into a unified agent format. Configure and monitor your connected channels.</p>

  <div class="channels-grid">
    {#each channels as ch (ch.name)}
      <article class="channel-card" class:running={ch.is_running} class:offline={!ch.is_running}>
        <div class="channel-header">
          <span class="channel-icon">{TYPE_ICONS[ch.channel_type] ?? "🔗"}</span>
          <div class="channel-info">
            <strong>{ch.name}</strong>
            <span class="channel-type">{ch.channel_type}</span>
          </div>
          <span class="status-dot" class:active={ch.is_running}></span>
        </div>

        <div class="channel-stats">
          <div class="stat">
            <span class="stat-value">{ch.messages_received}</span>
            <span class="stat-label">Received</span>
          </div>
          <div class="stat">
            <span class="stat-value">{ch.messages_sent}</span>
            <span class="stat-label">Sent</span>
          </div>
          <div class="stat">
            <span class="stat-value status-text" class:online={ch.is_running}>{ch.is_running ? "Online" : "Offline"}</span>
            <span class="stat-label">Status</span>
          </div>
        </div>

        {#if !ch.is_running}
          <div class="channel-config">
            <span class="config-hint">
              {#if ch.channel_type === "telegram"}
                Set <code>TELEGRAM_BOT_TOKEN</code> env var to connect
              {:else if ch.channel_type === "discord"}
                Set <code>DISCORD_BOT_TOKEN</code> env var to connect
              {:else if ch.channel_type === "slack"}
                Set <code>SLACK_BOT_TOKEN</code> env var to connect
              {:else}
                Configure in server settings
              {/if}
            </span>
          </div>
        {/if}
      </article>
    {/each}
  </div>

  <div class="gateway-info">
    <h3>Gateway Architecture</h3>
    <div class="flow">
      <span class="flow-step">Platform Message</span>
      <span class="flow-arrow">→</span>
      <span class="flow-step highlight">Gateway Normalizer</span>
      <span class="flow-arrow">→</span>
      <span class="flow-step">Agent Runtime</span>
      <span class="flow-arrow">→</span>
      <span class="flow-step highlight">Gateway Response</span>
      <span class="flow-arrow">→</span>
      <span class="flow-step">Platform Reply</span>
    </div>
  </div>
</section>

<style>
  .channels-app { height: 100%; overflow: auto; padding: 1.1rem; display: grid; gap: 1rem; align-content: start; }
  .header { display: flex; justify-content: space-between; align-items: center; gap: 1rem; }
  .eyebrow { margin: 0; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.14em; color: var(--system-color-text-muted); }
  h2, h3 { margin: 0.3rem 0 0; }
  .intro { margin: 0; font-size: 0.85rem; color: var(--system-color-text-muted); max-width: 600px; }
  .action-btn {
    border-radius: 999px; padding: 0.55rem 1rem; font-size: 0.82rem; cursor: pointer;
    border: 1px solid var(--system-color-border); background: var(--system-color-panel); color: var(--system-color-text);
  }
  .channels-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr)); gap: 0.85rem; }
  .channel-card {
    border-radius: 1.1rem; padding: 1rem; border: 1px solid var(--system-color-border);
    background: var(--system-color-panel); display: flex; flex-direction: column; gap: 0.75rem; transition: all 0.2s;
  }
  .channel-card.running { border-color: hsla(160 60% 50% / 0.4); }
  .channel-card.offline { opacity: 0.7; }
  .channel-header { display: flex; align-items: center; gap: 0.6rem; }
  .channel-icon { font-size: 1.4rem; }
  .channel-info { display: flex; flex-direction: column; flex: 1; }
  .channel-info strong { font-size: 0.88rem; }
  .channel-type { font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--system-color-text-muted); }
  .status-dot { width: 0.55rem; height: 0.55rem; border-radius: 50%; background: var(--system-color-text-muted); flex-shrink: 0; }
  .status-dot.active { background: var(--system-color-success); box-shadow: 0 0 6px hsla(160 60% 50% / 0.5); }
  .channel-stats { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 0.4rem; }
  .stat { display: flex; flex-direction: column; align-items: center; gap: 0.1rem; padding: 0.3rem 0; }
  .stat-value { font-size: 1.1rem; font-weight: 700; color: var(--system-color-text); }
  .stat-label { font-size: 0.68rem; color: var(--system-color-text-muted); text-transform: uppercase; letter-spacing: 0.08em; }
  .status-text { font-size: 0.78rem !important; }
  .status-text.online { color: var(--system-color-success); }
  .channel-config { font-size: 0.78rem; color: var(--system-color-text-muted); }
  .config-hint code { padding: 0.1rem 0.35rem; border-radius: 0.25rem; background: hsla(var(--system-color-dark-hsl) / 0.08); font-size: 0.75rem; }
  .gateway-info {
    margin-top: 0.5rem; padding: 1rem; border-radius: 1rem;
    background: hsla(var(--system-color-primary-hsl) / 0.04); border: 1px solid hsla(var(--system-color-primary-hsl) / 0.12);
  }
  .flow { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; margin-top: 0.6rem; }
  .flow-step {
    padding: 0.4rem 0.7rem; border-radius: 0.5rem; font-size: 0.78rem; font-weight: 500;
    background: var(--system-color-panel); border: 1px solid var(--system-color-border);
  }
  .flow-step.highlight { background: hsla(var(--system-color-primary-hsl) / 0.12); border-color: hsla(var(--system-color-primary-hsl) / 0.25); color: var(--system-color-primary); }
  .flow-arrow { color: var(--system-color-text-muted); font-size: 0.9rem; }
</style>
