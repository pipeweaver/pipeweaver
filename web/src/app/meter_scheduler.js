const callbacks = new Set();
let rafId = null;
let lastFrameTime = 0;
const frameInterval = 1000 / 40;

function tick(currentTime) {
  const elapsed = currentTime - lastFrameTime;

  if (elapsed >= frameInterval) {
    lastFrameTime = currentTime - (elapsed % frameInterval);
    for (const cb of callbacks) {
      cb(currentTime);
    }
  }

  rafId = callbacks.size > 0 ? requestAnimationFrame(tick) : null;
}

export default {
  register(callback) {
    callbacks.add(callback);
    if (rafId === null) {
      rafId = requestAnimationFrame(tick);
    }
  },

  unregister(callback) {
    callbacks.delete(callback);
  }
}
