(function () {
  const loader = document.getElementById("app-loader");
  const status = document.getElementById("loader-status");
  const canvas = document.getElementById("bevy-canvas");
  const pickerInput = document.getElementById("picker-keyboard-input");
  if (!loader) return;

  const viewportState = {
    width: window.innerWidth,
    height: window.innerHeight,
  };
  let pickerOpen = false;

  const applyViewportSizing = () => {
    const width = window.innerWidth;
    const height = window.innerHeight;
    const shouldUpdate =
      !pickerOpen || height >= viewportState.height || width !== viewportState.width;

    if (shouldUpdate) {
      viewportState.width = width;
      viewportState.height = height;

      const root = document.documentElement;
      const desktopCanvasHeight = Math.min(height * 0.8, 900);
      const mobileCanvasHeight = Math.min(height * 0.82, 900);

      root.style.setProperty("--app-viewport-height", `${height}px`);
      root.style.setProperty(
        "--app-canvas-height-desktop",
        `${desktopCanvasHeight}px`,
      );
      root.style.setProperty(
        "--app-canvas-height-mobile",
        `${mobileCanvasHeight}px`,
      );
    }
  };

  window.addEventListener("resize", () => {
    window.requestAnimationFrame(applyViewportSizing);
  });
  applyViewportSizing();

  const emitPickerQuery = (action) => {
    if (!pickerInput) return;
    const query = pickerInput.value || "";
    window.dispatchEvent(
      new CustomEvent("vizmat-picker-query", {
        detail: {
          query,
          action,
        },
      }),
    );
  };

  if (pickerInput) {
    window.addEventListener("vizmat-structure-picker-open", () => {
      pickerOpen = true;
      pickerInput.value = "";
      try {
        pickerInput.focus({ preventScroll: true });
      } catch (error) {
        pickerInput.focus();
        window.scrollTo(0, 0);
      }
      emitPickerQuery("change");
    });

    window.addEventListener("vizmat-structure-picker-close", () => {
      pickerOpen = false;
      pickerInput.blur();
      pickerInput.value = "";
      applyViewportSizing();
    });

    pickerInput.addEventListener("input", () => emitPickerQuery("change"));
    pickerInput.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        emitPickerQuery("submit");
        event.stopPropagation();
      }
    });
  }

  const emitTouchGesture = (gesture) => {
    window.dispatchEvent(
      new CustomEvent("vizmat-touch-gesture", {
        detail: gesture,
      }),
    );
  };

  const toVec = (touch) => ({ x: touch.clientX, y: touch.clientY });

  const touchPanState = {
    one: null,
    two: {
      active: false,
      midpoint: null,
      distance: null,
    },
  };

  const getMidpoint = (a, b) => ({
    x: (a.x + b.x) * 0.5,
    y: (a.y + b.y) * 0.5,
  });

  const getDistance = (a, b) => {
    const dx = a.x - b.x;
    const dy = a.y - b.y;
    return Math.hypot(dx, dy);
  };

  const clearTouchState = () => {
    touchPanState.one = null;
    touchPanState.two = {
      active: false,
      midpoint: null,
      distance: null,
    };
  };

  const emitIf = (condition, payload) => {
    if (!condition) return false;
    emitTouchGesture(payload);
    return true;
  };

  if (canvas) {
    canvas.style.touchAction = "none";

    if (window.PointerEvent) {
      const pointerState = new Map();

      const removePointer = (event) => {
        if (event.pointerType !== "touch") return;
        if (!event.currentTarget || !pointerState.has(event.pointerId)) return;
        pointerState.delete(event.pointerId);

        if (pointerState.size === 0) {
          clearTouchState();
          return;
        }

        if (pointerState.size === 1) {
          touchPanState.one = [...pointerState.values()][0];
          touchPanState.two = {
            active: false,
            midpoint: null,
            distance: null,
          };
          return;
        }

        const points = [...pointerState.values()];
        touchPanState.two = {
          active: true,
          midpoint: getMidpoint(points[0], points[1]),
          distance: getDistance(points[0], points[1]),
        };
      };

      const updateSinglePointer = (point) => {
        if (!touchPanState.one) {
          touchPanState.one = point;
          return;
        }

        const dx = point.x - touchPanState.one.x;
        const dy = point.y - touchPanState.one.y;
        const emitted = emitIf(dx !== 0 || dy !== 0, {
          gesture: "Rotate",
          dx,
          dy,
          scale_delta: 0,
        });
        if (emitted) {
          touchPanState.one = point;
        }
      };

      const updateTwoPointer = (a, b) => {
        if (!touchPanState.two.active) {
          touchPanState.two = {
            active: true,
            midpoint: getMidpoint(a, b),
            distance: getDistance(a, b),
          };
          return;
        }

        const midpoint = getMidpoint(a, b);
        const distance = getDistance(a, b);
        const panDx = midpoint.x - touchPanState.two.midpoint.x;
        const panDy = midpoint.y - touchPanState.two.midpoint.y;
        const scaleDelta =
          touchPanState.two.distance > 0
            ? (distance - touchPanState.two.distance) / touchPanState.two.distance
            : 0;

        const emitted = emitIf(panDx !== 0 || panDy !== 0 || scaleDelta !== 0, {
          gesture: "TwoFinger",
          dx: panDx,
          dy: panDy,
          scale_delta: scaleDelta,
        });

        if (emitted) {
          touchPanState.two.midpoint = midpoint;
          touchPanState.two.distance = distance;
        }
      };

      canvas.addEventListener(
        "pointerdown",
        (event) => {
          if (event.pointerType !== "touch") return;
          if (!event.target) return;
          const point = toVec(event);
          pointerState.set(event.pointerId, point);
          event.currentTarget.setPointerCapture(event.pointerId);
          touchPanState.one = point;
          touchPanState.two = {
            active: false,
            midpoint: null,
            distance: null,
          };
          event.preventDefault();
        },
        { passive: false },
      );

      canvas.addEventListener(
        "pointermove",
        (event) => {
          if (event.pointerType !== "touch") return;
          if (!pointerState.has(event.pointerId)) return;

          const point = toVec(event);
          pointerState.set(event.pointerId, point);

          if (pointerState.size === 1) {
            updateSinglePointer(point);
          } else if (pointerState.size >= 2) {
            const points = [...pointerState.values()];
            const a = points[0];
            const b = points[1];
            updateTwoPointer(a, b);
          }

          event.preventDefault();
        },
        { passive: false },
      );

      canvas.addEventListener(
        "pointerup",
        removePointer,
        { passive: false },
      );
      canvas.addEventListener("pointercancel", removePointer, {
        passive: false,
      });
      canvas.addEventListener("pointerleave", removePointer, {
        passive: false,
      });
      canvas.addEventListener("pointerout", removePointer, { passive: false });
      canvas.addEventListener("pointerlostcapture", removePointer);
    } else {
      canvas.addEventListener(
        "touchstart",
        (event) => {
          if (!event.target) return;
          if (event.touches.length === 1) {
            touchPanState.one = toVec(event.touches[0]);
            touchPanState.two.active = false;
          } else if (event.touches.length === 2) {
            const a = toVec(event.touches[0]);
            const b = toVec(event.touches[1]);
            touchPanState.two = {
              active: true,
              midpoint: getMidpoint(a, b),
              distance: getDistance(a, b),
            };
            touchPanState.one = null;
          }
          event.preventDefault();
        },
        { passive: false },
      );

      canvas.addEventListener(
        "touchmove",
        (event) => {
          if (event.touches.length === 1 && touchPanState.one) {
            const current = toVec(event.touches[0]);
            const dx = current.x - touchPanState.one.x;
            const dy = current.y - touchPanState.one.y;

            if (dx !== 0 || dy !== 0) {
              emitTouchGesture({
                gesture: "Rotate",
                dx,
                dy,
                scale_delta: 0,
              });
              touchPanState.one = current;
            }
            event.preventDefault();
            return;
          }

          if (event.touches.length === 2 && touchPanState.two.active) {
            const a = toVec(event.touches[0]);
            const b = toVec(event.touches[1]);
            const midpoint = getMidpoint(a, b);
            const distance = getDistance(a, b);

            const panDx = midpoint.x - touchPanState.two.midpoint.x;
            const panDy = midpoint.y - touchPanState.two.midpoint.y;
            const scaleDelta =
              touchPanState.two.distance > 0
                ? (distance - touchPanState.two.distance) / touchPanState.two.distance
                : 0;

            if (panDx !== 0 || panDy !== 0 || scaleDelta !== 0) {
              emitTouchGesture({
                gesture: "TwoFinger",
                dx: panDx,
                dy: panDy,
                scale_delta: scaleDelta,
              });
              touchPanState.two.midpoint = midpoint;
              touchPanState.two.distance = distance;
            }

            event.preventDefault();
          }
        },
        { passive: false },
      );
    }

    const clearTouch = () => clearTouchState();
    canvas.addEventListener("touchend", clearTouch, { passive: false });
    canvas.addEventListener("touchcancel", clearTouch, { passive: false });
    canvas.addEventListener("touchleave", clearTouch, { passive: false });
  }

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

  const setStatus = (message) => {
    if (status) {
      status.textContent = message;
    }
  };

  const startApp = () => {
    const bindings = window.wasmBindings;
    if (!bindings || typeof bindings.start !== "function") {
      setStatus("Waiting for WebAssembly bindings...");
      return false;
    }

    try {
      bindings.start();
      hideLoader();
      return true;
    } catch (error) {
      console.error("Failed to start WASM app:", error);
      setStatus("Failed to start WebAssembly app.");
      return false;
    }
  };

  // Trunk injects the WASM module and then fires this event.
  window.addEventListener(
    "TrunkApplicationStarted",
    () => {
      startApp();
    },
    { once: true },
  );

  // If Trunk fired before this script loaded, start immediately.
  if (window.wasmBindings?.start) {
    startApp();
  }

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
