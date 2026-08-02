import { onActivated, onDeactivated, onUnmounted } from 'vue';

export function useUnmountedOrDeactivated(callback: () => void) {
  let inactive = false;

  const run = () => {
    if (inactive) {
      return;
    }
    inactive = true;
    callback();
  };

  onDeactivated(run);
  onUnmounted(run);
  onActivated(() => {
    inactive = false;
  });
}
