<script lang="ts">
  import { onMount } from 'svelte';
  import { settings, settingsError, initSettings } from '$stores/settings';
  import SettingsPage from '$components/SettingsPage.svelte';

  let loading = $state(true);

  onMount(async () => {
    await initSettings();
    loading = false;
  });
</script>

<main class="min-h-screen bg-white text-gray-900 no-select">
  {#if loading}
    <div class="flex items-center justify-center h-screen">
      <p class="text-gray-400">Loading settings...</p>
    </div>
  {:else if $settingsError}
    <div class="p-6">
      <div class="bg-red-50 border border-red-200 rounded-lg p-4">
        <p class="text-red-700 font-medium">Failed to load settings</p>
        <p class="text-red-600 text-sm mt-1">{$settingsError}</p>
      </div>
    </div>
  {:else if $settings}
    <SettingsPage settings={$settings} />
  {/if}
</main>
