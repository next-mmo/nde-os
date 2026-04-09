<script lang="ts">
  import { useStore } from "../../../lib/use-store.svelte";
  import { shieldBrowserStore } from "../../../state/shield";
  import { onMount, onDestroy } from "svelte";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Card, CardContent, CardHeader, CardTitle, CardFooter } from "$lib/components/ui/card";
  import { Badge } from "$lib/components/ui/badge";

  const store = useStore(shieldBrowserStore);
  let pollInterval: ReturnType<typeof setInterval>;

  onMount(() => {
    store.loadDevices();
    pollInterval = setInterval(() => {
      if (!store.deviceActionBusy && !store.devicesLoading) {
        store.loadDevices();
      }
    }, 5000);
  });

  onDestroy(() => {
    clearInterval(pollInterval);
  });

  function deviceTypeIcon(type: string) {
    switch (type) {
      case "avd": return "🖥️";
      case "ldplayer": return "🎮";
      case "nox": return "🎮";
      case "usb": return "📱";
      case "tcp": return "🌐";
      default: return "📱";
    }
  }

  function deviceTypeLabel(type: string) {
    switch (type) {
      case "avd": return "Android Studio AVD";
      case "ldplayer": return "LDPlayer";
      case "nox": return "NoxPlayer";
      case "usb": return "USB Device";
      case "tcp": return "TCP/IP Device";
      default: return "Device";
    }
  }

  function statusColor(status: string) {
    switch (status) {
      case "online": return "bg-emerald-500";
      case "offline": return "bg-destructive";
      case "booting": return "bg-amber-500";
      case "unauthorized": return "bg-destructive";
      default: return "bg-muted-foreground";
    }
  }
</script>

<div class="flex flex-col gap-6 h-full text-foreground max-w-6xl mx-auto w-full">
  <div class="flex justify-between items-start pt-2">
    <div class="flex flex-col gap-1">
      <h3 class="text-2xl font-bold m-0 flex items-center gap-2">📱 Android Devices</h3>
      <p class="text-muted-foreground text-sm m-0">Manage connected physical and emulated devices for mobile profiles.</p>
    </div>
    <div class="flex gap-2">
      <Button variant="outline" onclick={() => store.loadDevices()} disabled={store.devicesLoading}>
        🔄 {store.devicesLoading ? "Scanning..." : "Refresh"}
      </Button>
      <Button onclick={() => store.setShowConnectDialog(true)}>
        🌐 Connect Remote
      </Button>
    </div>
  </div>

  {#if store.devicesError}
    <div class="flex items-center gap-2 p-3 bg-destructive/10 text-destructive border border-destructive/20 rounded-lg text-sm">
      <span>⚠️</span>
      <p class="m-0">{store.devicesError}</p>
    </div>
  {/if}

  <div class="grid grid-cols-[1fr_minmax(350px,1.2fr)] gap-6 flex-1 min-h-0 items-start">
    <!-- Active Devices List -->
    <div class="flex flex-col gap-6 overflow-y-auto pr-2 pb-6">
      <div class="flex flex-col gap-3">
        <h4 class="text-base font-semibold m-0 text-foreground">Active Devices</h4>
        {#if store.androidDevices.length === 0}
          <div class="p-8 text-center text-muted-foreground border border-dashed border-border rounded-xl bg-secondary/20">
            <p class="text-sm m-0">No active Android devices found. Start an emulator or plug in a phone via USB with USB Debugging enabled.</p>
          </div>
        {:else}
          <div class="flex flex-col gap-2">
            {#each store.androidDevices as device}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div
                class="flex items-center gap-4 p-3 border rounded-xl bg-card cursor-pointer transition-colors shadow-sm {store.selectedDevice?.serial === device.serial ? 'border-primary bg-primary/5 ring-1 ring-primary/20' : 'border-border hover:bg-secondary/50'}"
                onclick={() => store.setSelectedDevice(device)}
              >
                <div class="text-2xl bg-secondary/50 w-10 h-10 rounded-lg flex items-center justify-center border border-border/50">{deviceTypeIcon(device.device_type)}</div>
                <div class="flex flex-col flex-1 min-w-0">
                  <span class="text-sm font-semibold truncate text-foreground">{device.display_name}</span>
                  <span class="text-[11px] text-muted-foreground uppercase tracking-wider font-medium">{deviceTypeLabel(device.device_type)} • {device.serial}</span>
                </div>
                <div class="flex items-center gap-1.5 shrink-0">
                  <span class="w-2 h-2 rounded-full {statusColor(device.status)} shadow-sm"></span>
                  <span class="text-[10px] text-muted-foreground uppercase tracking-wider font-bold">{device.status}</span>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Emulators (AVDs) -->
      {#if store.avdList.length > 0}
        <div class="flex flex-col gap-3">
          <h4 class="text-base font-semibold m-0 text-foreground">Android Studio Emulators </h4>
          <div class="flex flex-col gap-2">
            {#each store.avdList as avd}
              <div class="flex items-center gap-3 p-2.5 px-3 bg-secondary/30 border border-border/60 rounded-xl shadow-sm">
                <span class="text-xl">🖥️</span>
                <span class="flex-1 text-sm font-medium text-foreground">{avd.name}</span>
                <Button variant="secondary" size="sm" class="h-7 text-xs shadow-none border-border" onclick={() => store.launchAvd(avd.name)} disabled={store.deviceActionBusy}>
                  ▶ Launch
                </Button>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>

    <!-- Active Device Detail / Control Panel -->
    <Card class="flex flex-col overflow-y-auto shadow-md border-border/60">
      {#if store.selectedDevice}
        {@const selected = store.selectedDevice}
        <CardHeader class="pb-3 border-b border-border/40 bg-secondary/10">
          <div class="flex justify-between items-center">
            <CardTitle class="text-lg">{selected.display_name}</CardTitle>
            <Badge variant={selected.status === 'online' ? 'default' : 'destructive'} class="px-2 py-0 h-5 text-[10px] uppercase tracking-wider">
              {selected.status}
            </Badge>
          </div>
        </CardHeader>
        <CardContent class="flex flex-col gap-6 pt-5">
          <div class="flex flex-col gap-1 p-3 bg-secondary/30 rounded-lg shadow-inner border border-border/40">
            <div class="flex justify-between text-sm py-1 border-b border-border/40 last:border-0">
              <span class="text-muted-foreground">Type</span>
              <span class="font-medium font-mono text-xs">{deviceTypeLabel(selected.device_type)}</span>
            </div>
            <div class="flex justify-between text-sm py-1 border-b border-border/40 last:border-0">
              <span class="text-muted-foreground">Serial</span>
              <span class="font-medium font-mono text-xs">{selected.serial}</span>
            </div>
          </div>

          <div class="flex flex-col gap-4">
            <h5 class="text-xs text-muted-foreground uppercase tracking-widest font-bold border-b border-border/50 pb-1 m-0">Quick Actions</h5>

            <!-- Open URL -->
            <div class="flex flex-col gap-2">
              <div class="flex gap-2">
                <Input type="text" placeholder="https://example.com" value={store.deviceUrlInput} oninput={e => store.setDeviceUrlInput(e.currentTarget.value)} class="flex-1 h-9 text-sm" />
                <Button variant="secondary" size="sm" class="h-9 px-3" onclick={() => store.openUrlOnDevice(selected.serial)} disabled={store.deviceActionBusy || !store.deviceUrlInput.trim()}>
                  🌐 Open URL
                </Button>
              </div>
            </div>

            <!-- Proxy Settings -->
            <div class="flex flex-col gap-2">
              <div class="flex gap-2">
                <Input type="text" placeholder="Proxy Host" value={store.deviceProxyHost} oninput={e => store.setDeviceProxyHost(e.currentTarget.value)} class="flex-1 h-9 text-sm" />
                <Input type="text" placeholder="Port" value={store.deviceProxyPort} oninput={e => store.setDeviceProxyPort(e.currentTarget.value)} class="w-20 h-9 text-sm text-center" />
              </div>
              <div class="flex gap-2 w-full">
                <Button variant="secondary" size="sm" class="flex-1 h-8" onclick={() => store.pushProxyToDevice(selected.serial)} disabled={store.deviceActionBusy || !store.deviceProxyHost.trim()}>
                  ✅ Set Proxy
                </Button>
                <Button variant="destructive" size="sm" class="flex-1 h-8 bg-destructive/10 text-destructive hover:bg-destructive/20 border-destructive/20" onclick={() => store.clearDeviceProxy(selected.serial)} disabled={store.deviceActionBusy}>
                  🗑 Clear
                </Button>
              </div>
            </div>

            <!-- Generic action buttons -->
            <div class="flex gap-2 pt-4 border-t border-border/50">
              <Button variant="outline" size="sm" class="flex-1" onclick={() => store.takeScreenshot(selected.serial)} disabled={store.deviceActionBusy}>
                📸 Screenshot
              </Button>
              {#if selected.is_emulator}
                <Button variant="destructive" size="sm" class="flex-1" onclick={() => store.stopDevice(selected.serial)} disabled={store.deviceActionBusy}>
                  ⏹ Power Off
                </Button>
              {/if}
            </div>

            {#if store.screenshotPath}
              <div class="p-3 text-[11px] text-emerald-500 bg-emerald-500/10 border border-emerald-500/20 rounded-md break-all font-mono">
                Saved to: {store.screenshotPath}
              </div>
            {/if}
          </div>
        </CardContent>
      {:else}
        <div class="flex flex-col items-center justify-center p-12 text-center text-muted-foreground h-full gap-4">
          <span class="text-6xl opacity-50">📱</span>
          <p class="text-sm max-w-[250px] m-0">Select an active device to manage proxies, launch URLs, or take screenshots.</p>
        </div>
      {/if}
    </Card>
  </div>
</div>

{#if store.showConnectDialog}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="fixed inset-0 bg-background/80 backdrop-blur-sm z-50 flex items-center justify-center" onclick={() => store.setShowConnectDialog(false)}>
    <div class="bg-card w-full max-w-sm rounded-xl border border-border/60 shadow-xl p-6" onclick={e => e.stopPropagation()}>
      <div class="flex flex-col gap-1.5 mb-5">
        <h3 class="text-lg font-bold m-0 text-foreground tracking-tight">Connect TCP Device</h3>
        <p class="text-sm text-muted-foreground m-0">Enter the IP address and port of a remote ADB device (e.g., LDPlayer instances/WSA).</p>
      </div>
      
      <Input type="text" placeholder="127.0.0.1:5555" value={store.connectAddress} oninput={e => store.setConnectAddress(e.currentTarget.value)} class="mb-6 font-mono text-sm h-10" />
      
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => store.setShowConnectDialog(false)}>Cancel</Button>
        <Button onclick={() => store.connectTcpDevice()} disabled={!store.connectAddress.trim() || store.deviceActionBusy}>Connect</Button>
      </div>
    </div>
  </div>
{/if}
