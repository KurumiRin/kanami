<script>
  // Line icons: 24 grid, 1.7 stroke width, the same set as the menu bar menu and the design mockup
  let { name } = $props();
  const PATHS = {
    overview: '<path d="M4 16a8 8 0 1 1 16 0"/><path d="M12 16l3.5-4.5"/><circle cx="12" cy="16" r="1.5" fill="currentColor" stroke="none"/>',
    remap: '<rect x="3" y="4" width="8" height="8" rx="2"/><rect x="13" y="12" width="8" height="8" rx="2"/><path d="M7 15v1.5a2 2 0 0 0 2 2h1.5M17 9V7.5a2 2 0 0 0-2-2h-1.5"/>',
    rules: '<path d="M4 8h14M15 5l3 3-3 3"/><path d="M20 16H6M9 13l-3 3 3 3"/>',
    layers: '<path d="M12 4l8.5 4.5L12 13 3.5 8.5z"/><path d="M3.5 12.5L12 17l8.5-4.5"/><path d="M3.5 16.5L12 21l8.5-4.5" opacity=".5"/>',
    combo: '<rect x="2.5" y="8" width="7.5" height="8" rx="2"/><rect x="14" y="8" width="7.5" height="8" rx="2"/><path d="M12 10.5v3M10.5 12h3"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    trash: '<path d="M5 7h14M10 4h4M7 7l1 13h8l1-13M10 11v5M14 11v5"/>',
    grip: '<path d="M9 6v.01M15 6v.01M9 12v.01M15 12v.01M9 18v.01M15 18v.01" stroke-width="2.4"/>',
    close: '<path d="M6 6l12 12M18 6L6 18"/>',
    events: '<path d="M3 12h3.5l2.5-6 4.5 12 2.5-6H21"/>',
    code: '<path d="M8.5 7.5L4 12l4.5 4.5M15.5 7.5L20 12l-4.5 4.5M13.5 5l-3 14"/>',
    keyboard: '<rect x="2.5" y="6" width="19" height="12" rx="2.5"/><path d="M6.5 10h1M10.5 10h1M14.5 10h1M17.5 10h.01M8 14h8"/>',
    access: '<circle cx="12" cy="5" r="1.7" fill="currentColor" stroke="none"/><path d="M5 9l7 1.5L19 9M12 10.5v4M12 14.5l-3 5.5M12 14.5l3 5.5"/>',
    lock: '<rect x="5" y="10.5" width="14" height="10" rx="2.5"/><path d="M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5"/>',
    check: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
    alert: '<circle cx="12" cy="12" r="9"/><path d="M12 7.5v5.5M12 16.5v.01"/>',
    history: '<path d="M4 12a8 8 0 1 0 2.4-5.7"/><path d="M4 4v4h4"/><path d="M12 8v4.5l3 2"/>',
    external: '<path d="M14 4h6v6M20 4l-9 9"/><path d="M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4"/>',
    pause: '<path d="M9 6v12M15 6v12"/>',
    play: '<path d="M8 5.5v13l10-6.5z"/>',
    power: '<path d="M12 3.5v8"/><path d="M7 6.5a7.5 7.5 0 1 0 10 0"/>',
    search: '<circle cx="11" cy="11" r="6.5"/><path d="M16 16l4 4"/>',
  };
</script>

<svg class="i" viewBox="0 0 24 24" aria-hidden="true">{@html PATHS[name] ?? ""}</svg>
