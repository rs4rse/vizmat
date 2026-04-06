(function () {
  const canvas = document.getElementById("bevy-canvas");

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

  if (canvas) {
    canvas.style.touchAction = "none";

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
      const scaleDelta = touchPanState.two.distance > 0
        ? (distance - touchPanState.two.distance) / touchPanState.two.distance
        : 0;

      if (panDx !== 0 || panDy !== 0 || scaleDelta !== 0) {
        emitTouchGesture({
          gesture: "TwoFinger",
          x: panDx,
          y: panDy,
          scale_delta: scaleDelta,
        });
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
        const x = point.x;
        const y = point.y;
        emitTouchGesture({
          gesture: "OneFingerDown",
          x,
          y,
          scale_delta: 0,
        });
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
          if (!touchPanState.one) {
            touchPanState.one = point;
            return;
          }

          const x = point.x;
          const y = point.y;
          emitTouchGesture({
            gesture: "OneFingerMove",
            x,
            y,
            scale_delta: 0,
          });
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
  }
})();
