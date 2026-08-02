import { onActivated, onMounted } from 'vue';

export function useMountedOrActivated(callback: () => void) {
  let initialActivation = true;

  onMounted(() => {
    callback();
  });

  onActivated(() => {
    if (initialActivation) {
      initialActivation = false;
      return;
    }
    callback();
  });
}
