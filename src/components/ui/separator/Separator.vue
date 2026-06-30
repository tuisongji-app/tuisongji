<script setup lang="ts">
import { cn } from '@/lib/utils'
import { Separator, type SeparatorProps } from 'radix-vue'

const props = defineProps<SeparatorProps & { class?: string }>()
</script>

<template>
  <Separator
    v-bind="props"
    :class="
      cn(
        'shrink-0 bg-border',
        props.orientation === 'vertical' ? 'h-full w-px' : 'h-px w-full',
        props.class,
      )
    "
  />
</template>
