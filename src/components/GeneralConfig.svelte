<script lang="ts">
  import type { AppSettings } from '$lib/types';

  let { settings, onUpdate }: { settings: AppSettings; onUpdate: (s: AppSettings) => void } =
    $props();

  function toggleStartWithSystem() {
    onUpdate({ ...settings, startWithSystem: !settings.startWithSystem });
  }

  function toggleNotifications() {
    onUpdate({ ...settings, showTrayNotifications: !settings.showTrayNotifications });
  }
</script>

<div class="space-y-4">
  <h2 class="text-sm font-semibold text-gray-700 uppercase tracking-wide">General</h2>

  <label class="flex items-center justify-between py-2">
    <div>
      <p class="text-sm font-medium text-gray-900">Start with system</p>
      <p class="text-xs text-gray-500">Launch Pisum Langue when you log in</p>
    </div>
    <button
      class="relative w-10 h-5 rounded-full transition-colors {settings.startWithSystem
        ? 'bg-blue-500'
        : 'bg-gray-300'}"
      onclick={toggleStartWithSystem}
      title="Toggle start with system"
    >
      <span
        class="absolute top-0.5 left-0.5 w-4 h-4 bg-white rounded-full shadow transition-transform {settings.startWithSystem
          ? 'translate-x-5'
          : ''}"
      ></span>
    </button>
  </label>

  <label class="flex items-center justify-between py-2">
    <div>
      <p class="text-sm font-medium text-gray-900">Show notifications</p>
      <p class="text-xs text-gray-500">Display OS notifications for errors and status</p>
    </div>
    <button
      class="relative w-10 h-5 rounded-full transition-colors {settings.showTrayNotifications
        ? 'bg-blue-500'
        : 'bg-gray-300'}"
      onclick={toggleNotifications}
      title="Toggle notifications"
    >
      <span
        class="absolute top-0.5 left-0.5 w-4 h-4 bg-white rounded-full shadow transition-transform {settings.showTrayNotifications
          ? 'translate-x-5'
          : ''}"
      ></span>
    </button>
  </label>
</div>
