<script lang="ts">
  import { fade, fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { desktop, toggleNotificationCenter } from "🍎/state/desktop.svelte";
  import { click_outside } from "🍎/actions";

  // Mock notifications
  const notifications = [
    { id: 1, app: 'Agent Core', title: 'Task Completed', message: 'The orchestrator agent successfully finished the data pipeline.', time: 'Just now', icon: '🤖' },
    { id: 2, app: 'App Store', title: 'Updates Available', message: 'Vibe Code Studio and 2 other apps have updates.', time: '2m ago', icon: '📦' },
    { id: 3, app: 'System', title: 'Low Memory', message: 'Consider closing background sessions to free up RAM.', time: '1h ago', icon: '⚠️' }
  ];

  function hide() {
    toggleNotificationCenter(false);
  }
</script>

<div 
  class="fixed top-0 bottom-0 right-0 w-[320px] pt-[2rem] z-[9000]"
  style="pointer-events: none;"
>
  <div 
    class="w-full h-full p-2 pointer-events-auto flex flex-col gap-2 relative bg-white/40 dark:bg-black/40 backdrop-blur-2xl border-l border-white/20 dark:border-black/20 shadow-[-10px_0_20px_rgba(0,0,0,0.1)] transition-colors"
    use:click_outside={hide}
    transition:fly={{ x: 100, duration: 250, easing: cubicOut }}
  >
    <div class="px-3 pt-2 pb-1 text-[13px] font-semibold text-gray-500 uppercase tracking-wider flex justify-between items-center">
      <span>Notifications</span>
      <button class="w-6 h-6 rounded-full hover:bg-black/10 dark:hover:bg-white/10 flex items-center justify-center transition-colors" onclick={hide}>✕</button>
    </div>

    <div class="flex-1 overflow-y-auto overflow-x-hidden flex flex-col gap-2 px-1 pb-4">
      {#each notifications as notif (notif.id)}
        <div class="bg-white/60 dark:bg-black/30 backdrop-blur-md rounded-xl p-3 shadow-sm border border-black/5 dark:border-white/5 flex flex-col gap-1.5 transition-transform hover:scale-[1.02] cursor-pointer" transition:fade={{ duration: 150 }}>
          <div class="flex justify-between items-center opacity-80 gap-2 text-[11px] font-medium text-black dark:text-white">
            <div class="flex items-center gap-1.5"><span class="text-sm">{notif.icon}</span> {notif.app}</div>
            <span class="opacity-50 font-normal">{notif.time}</span>
          </div>
          <strong class="text-[13px] font-semibold text-black dark:text-white leading-tight">{notif.title}</strong>
          <p class="text-[12px] text-gray-600 dark:text-gray-300 leading-snug">{notif.message}</p>
        </div>
      {/each}
    </div>

    <!-- Widgets Area -->
    <div class="px-3 pt-2 pb-1 text-[13px] font-semibold text-gray-500 uppercase tracking-wider">
      Widgets
    </div>
    
    <div class="bg-white/60 dark:bg-black/30 backdrop-blur-md rounded-xl p-4 shadow-sm border border-black/5 dark:border-white/5 mb-4 mx-1">
      <div class="text-[18px] font-semibold text-black dark:text-white text-center">
        {new Date().toLocaleDateString('en-US', { weekday: 'long' })}
      </div>
      <div class="text-[42px] font-light text-blue-500 text-center leading-none my-1">
        {new Date().getDate()}
      </div>
      <div class="text-[14px] font-medium text-gray-500 text-center uppercase tracking-widest">
        {new Date().toLocaleDateString('en-US', { month: 'long', year: 'numeric' })}
      </div>
    </div>
  </div>
</div>
