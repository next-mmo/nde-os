<script lang="ts">
  let p = $props();
</script>
<div style="display:flex;flex-direction:column;align-items:center;gap:0.5rem;padding:2rem;text-align:center">
  {#if p.icon}
    <span style="font-size:2.5rem">{p.icon}</span>
  {/if}
  <strong style="font-size:0.92rem">{p.title}</strong>
  {#if p.description}
    <p style="margin:0;font-size:0.82rem;color:var(--system-color-text-muted);max-width:280px">{p.description}</p>
  {/if}
  {#if p.actionLabel}
    <button style="margin-top:0.5rem;padding:0.45rem 1rem;border-radius:999px;background:hsl(215 90% 55%);color:#fff;border:none;font-size:0.8rem;cursor:pointer">
      {p.actionLabel}
    </button>
  {/if}
</div>