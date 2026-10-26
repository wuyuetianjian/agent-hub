(() => {
  let nextId = 0;
  const pending = new Map();
  const listeners = new Map();
  let closed = false;
  window.addEventListener('pagehide', () => {
    closed = true;
    for (const request of pending.values()) {
      clearTimeout(request.timer);
      request.reject(new Error('Overlay closed'));
    }
    pending.clear();
    listeners.clear();
  }, { once: true });
  Object.defineProperty(window, '__ORBIT_OVERLAY__', { value: Object.freeze({
    invoke(command, args = {}) {
      if (closed) return Promise.reject(new Error('Overlay closed'));
      return new Promise((resolve, reject) => {
        const id = ++nextId;
        // Directory selection is a human-paced native modal, not a stalled RPC.
        const timer = command === 'overlay_choose_directory' ? null : setTimeout(() => {
          pending.delete(id);
          reject(new Error('Overlay request timed out'));
        }, 30000);
        pending.set(id, { resolve, reject, timer });
        try { window.webkit.messageHandlers.orbit.postMessage({ id, command, args }); }
        catch (error) { clearTimeout(timer); pending.delete(id); reject(error); }
      });
    },
    async listen(event, callback) {
      if (!listeners.has(event)) listeners.set(event, new Set());
      listeners.get(event).add(callback);
      return () => {
        const callbacks = listeners.get(event);
        if (!callbacks) return;
        callbacks.delete(callback);
        if (!callbacks.size) listeners.delete(event);
      };
    }
  }) });
  Object.defineProperty(window, '__ORBIT_RECEIVE__', { value(message) {
    if (message.type === 'reply') {
      const request = pending.get(message.id);
      if (!request) return;
      pending.delete(message.id);
      clearTimeout(request.timer);
      if (message.ok) request.resolve(message.value);
      else request.reject(new Error(typeof message.value === 'string' ? message.value : JSON.stringify(message.value)));
    } else if (message.type === 'event') {
      for (const callback of [...(listeners.get(message.event) || [])]) {
        try { callback({ event: message.event, payload: message.payload }); }
        catch (error) { console.error('Overlay event callback failed', error); }
      }
    }
  } });
})();
