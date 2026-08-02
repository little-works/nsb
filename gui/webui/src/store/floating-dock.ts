import { defineStore } from 'pinia';
import { shallowRef, type VNodeChild } from 'vue';

export type FloatingDockContent = () => VNodeChild;

export const useFloatingDockStore = defineStore('floatingDock', () => {
  const content = shallowRef<FloatingDockContent | null>(null);

  function register(nextContent: FloatingDockContent) {
    content.value = nextContent;
    return () => {
      if (content.value === nextContent) {
        content.value = null;
      }
    };
  }

  return { content, register };
});
