<script lang="ts">
  import { useStore } from "../../../lib/use-store.svelte";
  import { shieldBrowserStore } from "../../../state/shield";
  import { engineIcon, engineLabel } from "./utils";
  import { Button } from "$lib/components/ui/button";
  import { Card, CardContent, CardHeader, CardTitle, CardDescription, CardFooter } from "$lib/components/ui/card";
  import { Badge } from "$lib/components/ui/badge";

  const store = useStore(shieldBrowserStore);
</script>

<div class="max-w-4xl mx-auto w-full flex flex-col gap-6 mt-4">
  <div class="flex flex-col gap-1">
    <h3 class="text-2xl font-bold tracking-tight m-0">Settings</h3>
    <p class="text-muted-foreground text-sm m-0">Manage installed antidetect engines and core preferences.</p>
  </div>

  <Card>
    <CardHeader>
      <CardTitle>Installed Engines</CardTitle>
      <CardDescription>Antidetect browser versions managed by Shield.</CardDescription>
    </CardHeader>
    <CardContent>
      {#if store.availableEngines.length === 0}
        <div class="p-8 text-center text-muted-foreground border border-dashed border-border rounded-lg bg-secondary/20">
          <p class="m-0 text-sm">No engines checked yet. Click "Check Default Engines" or wait for background sync.</p>
        </div>
      {:else}
        <div class="flex flex-col gap-3">
          {#each store.availableEngines as eng}
            <div class="flex items-center justify-between p-4 bg-secondary/30 border border-border/60 rounded-xl transition-colors hover:bg-secondary/50">
              <div class="flex items-center gap-4">
                <div class="text-3xl bg-background border border-border/50 w-12 h-12 flex items-center justify-center rounded-xl shadow-sm">
                  {eng.icon}
                </div>
                <div class="flex flex-col gap-0.5">
                  <div class="flex items-center gap-2">
                    <span class="font-semibold">{eng.name}</span>
                    <Badge variant={eng.available ? "default" : "secondary"} class="text-[10px] uppercase tracking-wider py-0 px-1.5 h-4">
                      {eng.available ? "Ready" : "Not Installed"}
                    </Badge>
                  </div>
                  <span class="text-xs text-muted-foreground">Version: {eng.version} • {eng.description}</span>
                </div>
              </div>
              
              <div class="flex items-center gap-2">
                {#if eng.available}
                  <Button 
                    variant="destructive" 
                    size="sm" 
                    class="font-semibold"
                    onclick={() => { if (confirm(`Remove ${eng.name} ${eng.version}? Profiles using this version will fail to launch.`)) store.removeEngine(eng.engine, eng.version); }} 
                    disabled={store.downloading}
                  >
                    🗑 Remove
                  </Button>
                {:else}
                  <Button 
                    variant="default" 
                    size="sm"
                    class="font-semibold shadow-sm"
                    onclick={() => { store.setSetupEngine(eng.engine); store.setSetupVersion(eng.version); store.installEngine(); }} 
                    disabled={store.downloading}
                  >
                    ⬇️ Install
                  </Button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </CardContent>
    <CardFooter class="bg-secondary/20 border-t border-border pt-4 px-6 pb-4 flex justify-between items-center">
      <p class="text-xs text-muted-foreground m-0">Engines are downloaded to <code>core/sandbox/.agents/shield/engines</code>.</p>
      {#if store.downloading}
        <div class="flex items-center gap-2 px-3 py-1.5 bg-primary/10 text-primary rounded-md text-sm font-medium border border-primary/20">
          <div class="w-3.5 h-3.5 border-2 border-primary border-t-transparent rounded-full animate-spin"></div>
          {Math.round(store.downloadProgress * 100)}% Downloading...
        </div>
      {:else}
        <Button 
          variant="outline" 
          size="sm"
          onclick={() => store.checkAvailableEngines()}
        >
          🔄 Refresh Status
        </Button>
      {/if}
    </CardFooter>
  </Card>
</div>
