<script lang="ts">
  let p = $props();
</script>
<label style="display:flex;align-items:center;gap:0.5rem;cursor:pointer;font-size:0.85rem">
  <input type="checkbox" style="accent-color:hsl(215 90% 55%)" />
  <span>{p.label}</span>
</label>