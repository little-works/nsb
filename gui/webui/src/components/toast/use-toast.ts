import { createApp, type App } from 'vue';
import type { ToastOptions } from './index.setup';
import ToastHost from './toast-host.setup';
import { showToast } from './toast-manager';
import { i18n } from '@/i18n';

export type ToastPresetOptions = Omit<ToastOptions, 'variant'>;

export interface ToastContext {
  show: (options: ToastOptions) => () => void;
  info: (options: ToastPresetOptions) => () => void;
  warn: (options: ToastPresetOptions) => () => void;
  error: (options: ToastPresetOptions) => () => void;
}

let toastApp: App | undefined;

function ensureToastHost() {
  if (toastApp || import.meta.env.SSR) {
    return;
  }
  const mountPoint = document.createElement('div');
  document.body.append(mountPoint);
  toastApp = createApp(ToastHost);
  toastApp.use(i18n);
  toastApp.mount(mountPoint);
}

const dismiss = () => {};

export const toast: ToastContext = {
  show(options) {
    if (import.meta.env.SSR) {
      return dismiss;
    }
    ensureToastHost();
    return showToast(options);
  },
  info(options) {
    return toast.show({ ...options, variant: 'info' });
  },
  warn(options) {
    return toast.show({ ...options, variant: 'warn' });
  },
  error(options) {
    return toast.show({ ...options, variant: 'error' });
  },
};

if (!import.meta.env.SSR) {
  // @ts-ignore
  window.__nsb_toast = toast;
}
