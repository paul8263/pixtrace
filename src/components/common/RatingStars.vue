<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    modelValue: number | null;
    readonly?: boolean;
    small?: boolean;
  }>(),
  { readonly: false, small: false },
);

const emit = defineEmits<{
  (event: "update:modelValue", value: number | null): void;
}>();

function choose(value: number) {
  if (props.readonly) return;
  emit("update:modelValue", props.modelValue === value ? null : value);
}
</script>

<template>
  <div
    class="rating-stars"
    :class="{ small, readonly }"
    role="group"
    aria-label="评级"
    @click.stop
  >
    <button
      v-for="value in 5"
      :key="value"
      class="rating-star"
      :class="{ filled: (modelValue ?? 0) >= value }"
      type="button"
      :disabled="readonly"
      :aria-label="`${value} 星`"
      :title="readonly ? undefined : '点击相同星级可清除评级'"
      @click.stop="choose(value)"
    >
      ★
    </button>
  </div>
</template>
