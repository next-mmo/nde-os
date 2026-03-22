<svelte:options runes={true} />

<script lang="ts">
  interface Props {
    app_id: string;
  }

  let { app_id }: Props = $props();

  const title = $derived(app_id === "app-store" ? "App Store" : "Terminal");
</script>

<section class="placeholder">
  <div class="hero">
    <img src="/app-icons/{app_id}/256.webp" alt="" />
    <div>
      <p class="eyebrow">Coming soon</p>
      <h2>{title}</h2>
      <p>{title} is reserved in the shell, but the actual app logic has not been added yet.</p>
    </div>
  </div>
</section>

<style>
  .placeholder {
    width: 100%;
    height: 100%;
    padding: 2rem;
    display: grid;
    place-items: center;
  }

  .hero {
    width: min(34rem, 100%);
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 1.4rem;
    align-items: center;
    padding: 1.5rem;
    border-radius: 1.4rem;
    background: var(--system-color-panel);
    border: 1px solid var(--system-color-border);
  }

  img {
    width: 5rem;
    height: 5rem;
  }

  .eyebrow {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    font-size: 0.72rem;
    color: var(--system-color-text-muted);
  }

  h2 {
    margin: 0.3rem 0 0.45rem;
    font-size: 1.6rem;
  }

  p:last-child {
    margin: 0;
    color: var(--system-color-text-muted);
  }
</style>
