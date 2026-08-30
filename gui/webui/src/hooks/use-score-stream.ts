import type {
  CoreApiConnectionsData,
  CoreApiLogsData,
  CoreApiTrafficData,
  CoreApiWsMessage,
  RuntimeStatus,
} from '@/types';
import { toast } from '@/components/toast';
import { i18n } from '@/i18n';
import { runtimeQueryKey } from '@/store/app';
import { useQueryClient } from '@tanstack/vue-query';
import { useDocumentVisibility } from '@vueuse/core';
import { onScopeDispose, readonly, ref, watch } from 'vue';

const RECONNECT_DELAY = 1_000;

const traffic = ref<CoreApiTrafficData>({ down: 0, up: 0 });
const connections = ref<CoreApiConnectionsData | null>(null);
const connectionState = ref<'connecting' | 'connected' | 'disconnected'>(
  'disconnected',
);
const failureNotificationVersion = ref(0);
const logListeners = new Set<(log: CoreApiLogsData | string) => void>();
const runtimeListeners = new Set<(runtime: RuntimeStatus) => void>();

let socket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | undefined;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function isRuntimeStatus(value: unknown): value is RuntimeStatus {
  if (!isRecord(value) || !isRecord(value.kernel)) {
    return false;
  }
  const kernel = value.kernel;
  return (
    typeof kernel.binary_path === 'string' &&
    typeof kernel.installed === 'boolean' &&
    typeof kernel.data_dir === 'string' &&
    typeof kernel.config_path === 'string' &&
    typeof kernel.version === 'string' &&
    typeof kernel.last_started_at === 'string' &&
    (kernel.status === 'Running' ||
      kernel.status === 'Stopped' ||
      kernel.status === 'Failed')
  );
}

function isTrafficData(value: unknown): value is CoreApiTrafficData {
  return (
    isRecord(value) &&
    typeof value.down === 'number' &&
    typeof value.up === 'number'
  );
}

function isConnectionsData(value: unknown): value is CoreApiConnectionsData {
  return (
    isRecord(value) &&
    typeof value.memory === 'number' &&
    typeof value.uploadTotal === 'number' &&
    typeof value.downloadTotal === 'number' &&
    Array.isArray(value.connections)
  );
}

function isLogsData(value: unknown): value is CoreApiLogsData | string {
  return (
    typeof value === 'string' ||
    (isRecord(value) &&
      typeof value.type === 'string' &&
      typeof value.payload === 'string')
  );
}

type ScoreMessage =
  | CoreApiWsMessage<'traffic'>
  | CoreApiWsMessage<'connections'>
  | CoreApiWsMessage<'logs'>
  | CoreApiWsMessage<'runtime'>;

function isScoreMessage(value: unknown): value is ScoreMessage {
  if (!isRecord(value) || typeof value.type !== 'string') {
    return false;
  }
  switch (value.type) {
    case 'traffic':
      return isTrafficData(value.data);
    case 'connections':
      return isConnectionsData(value.data);
    case 'logs':
      return isLogsData(value.data);
    case 'runtime':
      return isRuntimeStatus(value.data);
    default:
      return false;
  }
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
      if (!isScoreMessage(message)) {
        return;
      }
      switch (message.type) {
        case 'traffic':
          traffic.value = message.data;
          break;
        case 'connections':
          connections.value = message.data;
          break;
        case 'logs': {
          logListeners.forEach((listener) => listener(message.data));
          break;
        }
        case 'runtime': {
          runtimeListeners.forEach((listener) => listener(message.data));
          break;
        }
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

function subscribeRuntime(listener: (runtime: RuntimeStatus) => void) {
  runtimeListeners.add(listener);
  return () => runtimeListeners.delete(listener);
}

export function useScoreStreamConnection() {
  if (import.meta.env.SSR) {
    return;
  }

  const queryClient = useQueryClient();
  const visibility = useDocumentVisibility();
  const unsubscribeRuntime = subscribeRuntime((runtime) => {
    const previous = queryClient.getQueryData<RuntimeStatus>(runtimeQueryKey);
    queryClient.setQueryData(runtimeQueryKey, runtime);
    if (
      runtime.kernel.status === 'Failed' &&
      previous?.kernel.status !== 'Failed'
    ) {
      failureNotificationVersion.value += 1;
      toast.error({
        title: i18n.global.t('errors.kernelStartFailed'),
        content: i18n.global.t('errors.kernelStartFailedAction'),
      });
    }
  });
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
    unsubscribeRuntime();
    stop();
    disconnect();
  });
}

export function useScoreStreamData() {
  return {
    traffic: readonly(traffic),
    connections: readonly(connections),
    connectionState: readonly(connectionState),
    failureNotificationVersion: readonly(failureNotificationVersion),
    subscribeLogs: subscribeScoreLogs,
  };
}
