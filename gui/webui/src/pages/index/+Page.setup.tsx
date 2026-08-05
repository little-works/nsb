import { __render } from '@/shared/helpter';
import { useHomeStore } from '@/store/home';
import { storeToRefs } from 'pinia';
import OverviewPage from './overview-page.setup';

const homeStore = useHomeStore();
const {
  errorMessage,
  hideUnavailableNodes,
  kernelInstalled,
  kernelRunning,
  loading,
  proxyMode,
  proxyModeChanging,
  proxyGroups,
  searchKeyword,
  sortMode,
} = storeToRefs(homeStore);

defineOptions({ name: 'IndexRoutePage' });

export default __render(() => (
  <OverviewPage
    errorMessage={errorMessage.value}
    groups={proxyGroups.value}
    hideUnavailableNodes={hideUnavailableNodes.value}
    kernelInstalled={kernelInstalled.value}
    kernelRunning={kernelRunning.value}
    loading={loading.value}
    onProxyModeChange={homeStore.setProxyMode}
    onRefresh={homeStore.refreshProxyGroups}
    onHideUnavailableNodesChange={homeStore.setHideUnavailableNodes}
    onSearchKeywordChange={homeStore.setSearchKeyword}
    onSortModeCycle={homeStore.cycleSortMode}
    onSwitchProxy={homeStore.switchProxy}
    onTestProxyGroup={homeStore.testProxyGroup}
    searchKeyword={searchKeyword.value}
    proxyMode={proxyMode.value}
    proxyModeChanging={proxyModeChanging.value}
    sortMode={sortMode.value}
  />
));
