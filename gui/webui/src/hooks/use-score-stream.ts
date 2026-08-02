import type {
  CoreApiConnectionsData,
  CoreApiLogsData,
  CoreApiTrafficData,
} from '@/types';
import { useDocumentVisibility } from '@vueuse/core';
import { onScopeDispose, readonly, ref, watch } from 'vue';

const RECONNECT_DELAY = 1_000;

const traffic = ref<CoreApiTrafficData>({ down: 0, up: 0 });
const connections = ref<CoreApiConnectionsData | null>(null);
const connectionState = ref<'connecting' | 'connected' | 'disconnected'>(
  'disconnected',
);
const logListeners = new Set<(log: CoreApiLogsData | string) => void>();

let socket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | undefined;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function scheduleReconnect() {
  if (
    reconnectTimer ||
    import.meta.env.SSR ||
    document.visibilityState !== 'visible'
  ) {
    return;
  }
  reconnectTimer = setTimeout(() => {
    reconnectTimer = undefined;
    connect();
  }, RECONNECT_DELAY);
}

function connect() {
  if (
    import.meta.env.SSR ||
    socket?.readyState === WebSocket.CONNECTING ||
    socket?.readyState === WebSocket.OPEN
  ) {
    return;
  }

  connectionState.value = 'connecting';
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const nextSocket = new WebSocket(
    `${protocol}//${window.location.host}/api/score/stream`,
  );
  socket = nextSocket;
  nextSocket.onopen = () => {
    if (socket === nextSocket) {
      connectionState.value = 'connected';
    }
  };
  nextSocket.onmessage = (event) => {
    if (typeof event.data !== 'string') {
      return;
    }
    try {
      const message: unknown = JSON.parse(event.data);
      if (!isRecord(message) || typeof message.type !== 'string') {
        return;
      }
      if (message.type === 'traffic' && isRecord(message.data)) {
        traffic.value = message.data as unknown as CoreApiTrafficData;
      } else if (message.type === 'connections' && isRecord(message.data)) {
        connections.value = message.data as unknown as CoreApiConnectionsData;
      } else if (message.type === 'logs') {
        const log = message.data as CoreApiLogsData | string;
        logListeners.forEach((listener) => listener(log));
      }
    } catch (error) {
      console.error('Failed to parse score stream message:', error);
    }
  };
  nextSocket.onclose = () => {
    if (socket !== nextSocket) {
      return;
    }
    socket = null;
    connectionState.value = 'disconnected';
    scheduleReconnect();
  };
  nextSocket.onerror = () => {
    if (socket === nextSocket) {
      connectionState.value = 'disconnected';
    }
  };
}

function disconnect() {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = undefined;
  }
  socket?.close();
  socket = null;
  connectionState.value = 'disconnected';
}

function subscribeScoreLogs(listener: (log: CoreApiLogsData | string) => void) {
  logListeners.add(listener);
  return () => logListeners.delete(listener);
}

export function useScoreStreamConnection() {
  if (import.meta.env.SSR) {
    return;
  }

  const visibility = useDocumentVisibility();
  const stop = watch(
    visibility,
    (state) => {
      if (state === 'visible') {
        connect();
      } else {
        disconnect();
      }
    },
    { immediate: true },
  );
  onScopeDispose(() => {
    stop();
    disconnect();
  });
}

export function useScoreStreamData() {
  return {
    traffic: readonly(traffic),
    connections: readonly(connections),
    connectionState: readonly(connectionState),
    subscribeLogs: subscribeScoreLogs,
  };
}
