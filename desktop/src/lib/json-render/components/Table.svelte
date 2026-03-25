<script lang="ts">
  let p = $props();
</script>
<table style="width:100%;border-collapse:collapse">
  <thead>
    <tr>
      {#each (p.headers ?? []) as h}
        <th style="padding:0.5rem 0.75rem;text-align:left;font-size:0.72rem;text-transform:uppercase;letter-spacing:0.08em;color:var(--system-color-text-muted);border-bottom:1px solid var(--system-color-border)">{h}</th>
      {/each}
    </tr>
  </thead>
  <tbody>
    {#each (p.rows ?? []) as row, i}
      <tr style="{p.striped && i % 2 ? 'background:hsla(0 0% 50%/0.04)' : ''}">
        {#each row as cell}
          <td style="padding:0.5rem 0.75rem;font-size:0.82rem;border-bottom:1px solid var(--system-color-border)">{cell}</td>
        {/each}
      </tr>
    {/each}
  </tbody>
</table>