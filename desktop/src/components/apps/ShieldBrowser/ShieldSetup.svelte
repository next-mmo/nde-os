<script lang="ts">
  import { useStore } from "../../../lib/use-store.svelte";
  import { shieldBrowserStore } from "../../../state/shield";
  import { Button } from "$lib/components/ui/button";
  import { Card, CardContent, CardHeader, CardTitle, CardDescription, CardFooter } from "$lib/components/ui/card";
  
  const store = useStore(shieldBrowserStore);
</script>

<div class="flex flex-col items-center justify-center min-h-[500px] w-full p-6 text-foreground bg-background">
  <div class="mb-8 text-6xl drop-shadow-md">🛡️</div>
  <h2 class="text-3xl font-bold tracking-tight mb-3 text-center">Welcome to Shield Browser</h2>
  <p class="text-muted-foreground max-w-lg text-center mb-10 text-[15px] leading-relaxed">
    Shield uses customized, privacy-hardened browser engines (Camoufox / Wayfern) to provide 
    isolated environments that completely bypass advanced bot detection algorithms.
  </p>

  <Card class="w-full max-w-lg shadow-xl border-border bg-card">
    <CardHeader class="pb-4">
      <CardTitle class="text-xl">Engine Required</CardTitle>
      <CardDescription>To create your first profile, we need to download a core browser engine.</CardDescription>
    </CardHeader>
    <CardContent class="bg-secondary/20 m-4 rounded-xl p-4 border border-border">
      <p class="text-sm font-semibold mb-3 flex items-center gap-2 m-0 text-foreground">
        Choose Engine:
      </p>
      
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div 
        class="flex gap-4 p-4 border rounded-lg cursor-pointer transition-all hover:bg-secondary/50 {store.setupEngine === 'camoufox' ? 'border-primary bg-primary/5 ring-1 ring-primary/50' : 'border-border bg-background'}"
        onclick={() => store.setSetupEngine("camoufox")}
      >
        <div class="text-4xl">🦊</div>
        <div class="flex flex-col gap-1">
          <span class="font-bold text-foreground">Camoufox</span>
          <span class="text-xs text-muted-foreground leading-snug">Firefox-based engine specialized in WebRTC leaks and Canvas fingerprinting defense. Recommended.</span>
        </div>
      </div>
    </CardContent>
    <CardFooter class="flex flex-col p-6 pt-0 gap-4">
      {#if store.downloading}
        <div class="w-full">
          <div class="flex justify-between items-center text-sm font-medium mb-2 text-foreground">
            <span>Downloading engine...</span>
            <span class="text-primary">{Math.round(store.downloadProgress * 100)}%</span>
          </div>
          <div class="w-full h-2.5 bg-secondary rounded-full overflow-hidden shadow-inner">
            <div 
              class="h-full bg-primary rounded-full transition-all duration-300 ease-out relative"
              style:width="{store.downloadProgress * 100}%"
            >
              <div class="absolute inset-0 bg-white/20"></div>
            </div>
          </div>
        </div>
      {:else if store.setupError}
        <div class="w-full text-xs text-destructive bg-destructive/10 border border-destructive/20 p-3 rounded-md flex items-center gap-2">
          <span>⚠️</span> <span>{store.setupError}</span>
        </div>
        <Button class="w-full h-11 text-base shadow-md" onclick={() => store.installEngine()} disabled={store.resolvingVersion}>
          {store.resolvingVersion ? "Resolving latest version..." : "Retry Download"}
        </Button>
      {:else}
        <Button class="w-full h-11 text-base font-semibold shadow-md" onclick={() => store.installEngine()} disabled={store.resolvingVersion}>
          {store.resolvingVersion ? "Resolving latest version..." : "Download & Install Engine"}
        </Button>
        <p class="text-[11px] text-muted-foreground text-center uppercase tracking-wider font-medium">approx. 120MB download</p>
      {/if}
    </CardFooter>
  </Card>
</div>
