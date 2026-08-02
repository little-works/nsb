<!-- https://vike.dev/Layout -->

<template>
  <component :is="render()" />
</template>

<script setup lang="tsx">
import '../tailwind.css';
import { KeepAlive, useSlots } from 'vue';
import NsbLayout from '../components/nsb-layout.setup';

defineOptions({
  name: 'Layout',
});

const slots = useSlots();

const render = () => (
  <div id="application">
    <NsbLayout>{slots.default?.()}</NsbLayout>
  </div>
);
</script>
