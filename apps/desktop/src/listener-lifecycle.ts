export type ListenerCleanup = () => void;

export type ListenerRegistration = {
  register: (cleanup: ListenerCleanup) => void;
  dispose: () => void;
};

export function createListenerRegistration(): ListenerRegistration {
  let disposed = false;
  let cleanup: ListenerCleanup | null = null;
  return {
    register(nextCleanup) {
      if (disposed) nextCleanup();
      else cleanup = nextCleanup;
    },
    dispose() {
      disposed = true;
      cleanup?.();
      cleanup = null;
    },
  };
}
