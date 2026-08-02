import { ref } from 'vue';
import type { ToastOptions } from './index.setup';

export interface ToastEntry extends ToastOptions {
  id: number;
}

export const toasts = ref<ToastEntry[]>([]);

const timers = new Map<number, ReturnType<typeof setTimeout>>();
const remainingDurations = new Map<number, number>();
const timerStartedAt = new Map<number, number>();
let nextToastId = 0;

export function dismissToast(id: number) {
  const timer = timers.get(id);
  if (timer) {
    clearTimeout(timer);
    timers.delete(id);
  }
  remainingDurations.delete(id);
  timerStartedAt.delete(id);
  toasts.value = toasts.value.filter((toast) => toast.id !== id);
}

function startTimer(id: number, duration: number) {
  if (duration <= 0) {
    return;
  }
  timerStartedAt.set(id, Date.now());
  timers.set(
    id,
    setTimeout(() => dismissToast(id), duration),
  );
}

export function pauseToast(id: number) {
  const timer = timers.get(id);
  if (!timer) {
    return;
  }
  clearTimeout(timer);
  timers.delete(id);
  const elapsed = Date.now() - (timerStartedAt.get(id) ?? Date.now());
  remainingDurations.set(
    id,
    Math.max(0, (remainingDurations.get(id) ?? 0) - elapsed),
  );
  timerStartedAt.delete(id);
}

export function resumeToast(id: number) {
  const duration = remainingDurations.get(id);
  if (!duration) {
    return;
  }
  startTimer(id, duration);
}

export function showToast(options: ToastOptions) {
  const id = nextToastId;
  nextToastId += 1;
  const duration = Math.max(
    0,
    options.duration ?? (options.content ? 5000 : 3000),
  );
  toasts.value.push({
    id,
    ...options,
    closable: options.closable ?? duration === 0,
    duration,
  });
  if (duration > 0) {
    remainingDurations.set(id, duration);
    startTimer(id, duration);
  }
  return () => dismissToast(id);
}
