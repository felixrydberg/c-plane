<script setup lang="ts">
const container = ref<HTMLElement>()

const lockHeight = (element: Element) => {
  if (!container.value || !(element instanceof HTMLElement)) return

  container.value.style.height = `${element.offsetHeight}px`
}

const transitionHeight = (element: Element) => {
  if (!container.value || !(element instanceof HTMLElement)) return

  // ponytail: animates height only during rare step changes; use FLIP if this becomes high-frequency content.
  requestAnimationFrame(() => {
    container.value!.style.height = `${element.offsetHeight}px`
  })
}

const releaseHeight = () => {
  if (container.value) container.value.style.height = ''
}
</script>

<template>
  <div ref="container" class="ui-content-transition">
    <Transition name="ui-content" mode="out-in" @before-leave="lockHeight" @enter="transitionHeight" @after-enter="releaseHeight">
      <slot />
    </Transition>
  </div>
</template>

<style>
.ui-content-transition {
  transition: height 180ms cubic-bezier(0.23, 1, 0.32, 1);
}

.ui-content-enter-active,
.ui-content-leave-active {
  will-change: opacity, transform;
  transition-property: opacity, transform;
  transition-timing-function: cubic-bezier(0.23, 1, 0.32, 1);
}

.ui-content-enter-active {
  transition-duration: 180ms;
}

.ui-content-leave-active {
  transition-duration: 120ms;
}

.ui-content-enter-from,
.ui-content-leave-to {
  opacity: 0;
  transform: scale(0.985);
}

@media (prefers-reduced-motion: reduce) {
  .ui-content-enter-active,
  .ui-content-leave-active {
    transition-duration: 120ms;
    transition-property: opacity;
  }

  .ui-content-transition {
    transition-duration: 120ms;
  }

  .ui-content-enter-from,
  .ui-content-leave-to {
    transform: none;
  }
}
</style>
