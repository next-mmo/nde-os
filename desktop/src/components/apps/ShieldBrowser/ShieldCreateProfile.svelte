<script lang="ts">
  import { useStore } from "../../../lib/use-store.svelte";
  import { shieldBrowserStore } from "../../../state/shield";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import * as Select from "$lib/components/ui/select";
  import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "$lib/components/ui/card";

  const store = useStore(shieldBrowserStore);
</script>

<div class="max-w-md mx-auto mt-8 w-full">
  <Card>
    <CardHeader>
      <CardTitle>Create New Profile</CardTitle>
      <CardDescription>Configure a new isolated browser instance.</CardDescription>
    </CardHeader>
    <CardContent class="grid gap-6">
      <div class="grid gap-2">
        <label for="profile-name" class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Profile Name</label>
        <Input 
          id="profile-name" 
          placeholder="e.g., US Firefox Business" 
          value={store.newName} 
          oninput={(e) => store.setNewName(e.currentTarget.value)} 
        />
      </div>

      <div class="grid gap-2">
        <label class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Browser Engine</label>
        <Select.Root 
          type="single"
          value={store.newEngine} 
          onValueChange={(val) => store.setNewEngine(val)}
        >
          <Select.Trigger class="w-full">
            {#if store.newEngine}
              {@const eng = store.availableEngines.find(e => e.engine === store.newEngine)}
              {#if eng}
                <span class="flex items-center gap-2">{eng.icon} {eng.name}</span>
              {:else}
                Select an engine
              {/if}
            {:else}
              Select an engine
            {/if}
          </Select.Trigger>
          <Select.Content>
            {#each store.availableEngines.filter(e => e.available) as eng (eng.engine)}
              <Select.Item value={eng.engine}>
                <span class="flex items-center gap-2">{eng.icon} {eng.name}</span>
              </Select.Item>
            {/each}
          </Select.Content>
        </Select.Root>
      </div>

      <div class="grid gap-2">
        <label for="version-input" class="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Engine Version</label>
        <Input 
          id="version-input" 
          value={store.newVersion} 
          readonly
          class="bg-muted opacity-80 cursor-not-allowed"
          placeholder={store.resolvingVersion ? "Resolving..." : "Version"} 
        />
      </div>

      <div class="my-2 p-3 bg-secondary/50 rounded-lg text-sm border border-border/50">
        {#each store.availableEngines.filter(e => e.engine === store.newEngine) as eng}
          <p class="m-0 flex items-center gap-2">
            <span class="text-lg">{eng.icon}</span> 
            <span><strong>{eng.name}</strong> &mdash; {eng.description}</span>
          </p>
        {/each}
      </div>

      <Button 
        class="w-full font-semibold" 
        onclick={() => store.createProfile()} 
        disabled={!store.newName.trim() || !store.newVersion || store.resolvingVersion}
      >
        {store.resolvingVersion ? "Resolving version..." : "Create Profile"}
      </Button>
    </CardContent>
  </Card>
</div>
