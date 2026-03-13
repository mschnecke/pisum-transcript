<script lang="ts">
  import { onMount } from 'svelte';
  import type { HotkeyBinding } from '$lib/types';

  let {
    onCapture,
    onCancel,
  }: { onCapture: (binding: HotkeyBinding) => void; onCancel: () => void } = $props();

  let activeModifiers = $state<Set<string>>(new Set());
  let capturedKey = $state<string | null>(null);
  let element: HTMLDivElement | undefined = $state();

  const MODIFIER_KEYS: Record<string, string> = {
    Control: 'Ctrl',
    Shift: 'Shift',
    Alt: 'Alt',
    Meta: 'Cmd',
  };

  function mapKeyName(key: string): string {
    if (key.length === 1) return key.toUpperCase();
    if (key.startsWith('Arrow')) return key.slice(5);
    if (key === ' ') return 'Space';
    return key;
  }

  function handleKeyDown(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();

    if (e.key === 'Escape') {
      onCancel();
      return;
    }

    if (e.key in MODIFIER_KEYS) {
      activeModifiers = new Set([...activeModifiers, MODIFIER_KEYS[e.key]]);
      return;
    }

    // Non-modifier key: require at least one modifier
    if (activeModifiers.size === 0) return;

    capturedKey = mapKeyName(e.key);
    const binding: HotkeyBinding = {
      modifiers: [...activeModifiers],
      key: capturedKey,
    };
    onCapture(binding);
  }

  function handleKeyUp(e: KeyboardEvent) {
    e.preventDefault();
    if (e.key in MODIFIER_KEYS) {
      const next = new Set(activeModifiers);
      next.delete(MODIFIER_KEYS[e.key]);
      activeModifiers = next;
    }
  }

  onMount(() => {
    element?.focus();
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  bind:this={element}
  class="px-4 py-4 bg-blue-50 border-2 border-blue-300 border-dashed rounded-lg text-center focus:outline-none"
  role="textbox"
  tabindex="0"
  onkeydown={handleKeyDown}
  onkeyup={handleKeyUp}
>
  <p class="text-sm text-blue-700 font-medium">
    {#if activeModifiers.size > 0}
      {[...activeModifiers].join(' + ')} + ...
    {:else}
      Press a key combination...
    {/if}
  </p>
  <p class="text-xs text-blue-500 mt-1">Hold modifier(s) then press a key. Escape to cancel.</p>
</div>
