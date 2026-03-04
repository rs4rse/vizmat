(function () {
  const canvas = document.getElementById("bevy-canvas");
  const loader = document.getElementById("app-loader");
  const status = document.getElementById("loader-status");
  if (!loader) return;

  if (canvas) {
    // Keep RMB free for in-app panning instead of opening browser context menu.
    canvas.addEventListener("contextmenu", (event) => event.preventDefault());
    // Ensure keyboard/mouse interactions target the Bevy canvas after any click.
    canvas.addEventListener("mousedown", () => canvas.focus());
  }

  const hideLoader = () => {
    loader.classList.add("hidden");
    window.setTimeout(() => loader.remove(), 260);
  };

  // Trunk injects a startup script and emits this event after WASM init.
  // If the event fired before this listener was added, hide immediately.
  if (window.wasmBindings) {
    hideLoader();
  } else {
    window.addEventListener("TrunkApplicationStarted", hideLoader, {
      once: true,
    });
  }

  // Fallback so users are not stuck forever if startup event never fires.
  window.setTimeout(() => {
    if (!loader.classList.contains("hidden") && status) {
      status.textContent = "Still loading... first run can take longer.";
    }
  }, 9000);
})();
