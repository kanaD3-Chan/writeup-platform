<script setup lang="ts">
withDefaults(defineProps<{ name: keyof typeof paths; size?: number }>(), { size: 20 })

const paths = {
  language: 'M3 5h12M9 3v2m4 0c-1 7-5 10-10 12m2-9c1 4 4 7 8 9m0 4 4-11 4 11m-6-4h6',
  training: 'M6 5v14M3 8v8m3-4h12m0-7v14m3-11v8',
  flag: 'M5 21V4m0 0c5-4 9 4 14 0v10c-5 4-9-4-14 0',
  file: 'M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8zm0 0v6h6M8 13h8m-8 4h6',
  book: 'M12 7v14M3 3h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5v16h-5a4 4 0 0 0-4 2 4 4 0 0 0-4-2H3z',
  upload: 'M12 16V3m-5 5 5-5 5 5M4 16v4a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-4',
  download: 'M12 3v13m-5-5 5 5 5-5M4 16v4a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-4',
  user: 'M16 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0M4 21v-2a6 6 0 0 1 6-6h4a6 6 0 0 1 6 6v2',
  logout: 'M9 4H4v16h5m5-13 5 5-5 5m-6-5h12',
  sun: 'M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5',
  moon: 'M20.8 13A9 9 0 0 1 11 3.2 9 9 0 1 0 20.8 13Z',
  monitor: 'M4 3h16a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1m8 14v4m-4 0h8',
  arrowLeft: 'm14 6-6 6 6 6M8 12h13',
  arrowRight: 'm10 6 6 6-6 6M3 12h13',
  check: 'm5 12 4 4L19 6',
  close: 'm6 6 12 12M6 18 18 6',
  clock: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0m-9-5v5l3 2',
  folder: 'M3 6a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z',
  settings: 'M12 8a4 4 0 1 1 0 8 4 4 0 0 1 0-8m-2-5h4l1 3 3 1 3-1 2 4-2 2v3l2 2-2 4-3-1-3 1-1 3h-4l-1-3-3-1-3 1-2-4 2-2v-3L1 10l2-4 3 1 3-1Z',
  search: 'M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0m-2 5 6 6',
  menu: 'M4 6h16M4 12h16M4 18h16',
  key: 'M14 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0m-4 4v10m0-5h4m-4 3h3',
  pin: 'M12 17v5M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z',
} as const
</script>

<template>
  <svg :width="size" :height="size" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path :d="paths[name]" /></svg>
</template>
