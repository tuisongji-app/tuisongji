<script setup lang="ts">
import { cn } from '@/lib/utils'

const props = defineProps<{
  class?: string
  defaultValue?: number
  modelValue?: number
  min?: number
  max?: number
  step?: number
  disabled?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', val: number): void
}>()
</script>

<template>
  <div
    :class="cn(
      'inline-flex items-stretch rounded-md border border-input bg-transparent h-9',
      props.disabled && 'opacity-50 cursor-not-allowed',
      props.class,
    )"
  >
    <button
      type="button"
      class="flex items-center justify-center w-8 border-r border-input text-sm text-muted-foreground hover:bg-accent disabled:cursor-not-allowed"
      :disabled="disabled || (min != null && (modelValue ?? defaultValue ?? 0) <= min)"
      @click="emit('update:modelValue', (modelValue ?? defaultValue ?? 0) - (step ?? 1))"
    >
      -
    </button>
    <input
      type="number"
      :value="modelValue ?? defaultValue"
      :min="min"
      :max="max"
      :step="step"
      :disabled="disabled"
      class="w-16 bg-transparent text-center text-sm outline-none [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
      @input="emit('update:modelValue', Number(($event.target as HTMLInputElement).value))"
    />
    <button
      type="button"
      class="flex items-center justify-center w-8 border-l border-input text-sm text-muted-foreground hover:bg-accent disabled:cursor-not-allowed"
      :disabled="disabled || (max != null && (modelValue ?? defaultValue ?? 0) >= max)"
      @click="emit('update:modelValue', (modelValue ?? defaultValue ?? 0) + (step ?? 1))"
    >
      +
    </button>
  </div>
</template>
