<script setup lang="ts">
// 动态任务表单组件：根据 schema 渲染字段
// 防御性设计：新增字段类型只需在这里扩展

import { computed } from 'vue'
import type { TaskField } from '../config/tasks'

const props = defineProps<{
  fields: TaskField[]
  modelValue: Record<string, unknown>
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: Record<string, unknown>): void
}>()

const model = computed({
  get: () => props.modelValue,
  set: (v: Record<string, unknown>) => emit('update:modelValue', v),
})

function setValue(name: string, value: unknown) {
  model.value = { ...model.value, [name]: value }
}

function parseArray(text: string): string[] {
  return text.split(/[,，]/).map((s) => s.trim()).filter(Boolean)
}

function arrayToText(v: unknown): string {
  if (Array.isArray(v)) return v.join(', ')
  if (typeof v === 'string') return v
  return ''
}

// 多选项的值转换（保存为数组）
function toggleArrayValue(name: string, value: string | number, checked: boolean) {
  const current = Array.isArray(model.value[name]) ? (model.value[name] as unknown[]) : []
  const next = checked
    ? [...current, value]
    : current.filter((v) => v !== value)
  setValue(name, next)
}

function isArraySelected(name: string, value: string | number): boolean {
  const arr = model.value[name]
  return Array.isArray(arr) && arr.includes(value as never)
}
</script>

<template>
  <div class="task-form">
    <div v-for="field in fields" :key="field.name" class="form-field">
      <label class="form-label">
        {{ field.label }}
        <span v-if="field.required" class="required">*</span>
      </label>

      <!-- 数字输入 -->
      <input
        v-if="field.type === 'number'"
        :value="(model[field.name] as number) ?? 0"
        type="number"
        class="input"
        @input="setValue(field.name, Number(($event.target as HTMLInputElement).value))"
      />

      <!-- 文本输入 -->
      <input
        v-else-if="field.type === 'string'"
        :value="(model[field.name] as string) ?? ''"
        type="text"
        class="input"
        :placeholder="field.placeholder || ''"
        @input="setValue(field.name, ($event.target as HTMLInputElement).value)"
      />

      <!-- 布尔开关 -->
      <label v-else-if="field.type === 'boolean'" class="switch">
        <input
          type="checkbox"
          :checked="!!model[field.name]"
          @change="setValue(field.name, ($event.target as HTMLInputElement).checked)"
        />
        <span class="switch-slider"></span>
        <span class="switch-text">{{ model[field.name] ? '开' : '关' }}</span>
      </label>

      <!-- 单选下拉 -->
      <select
        v-else-if="field.type === 'select'"
        :value="String(model[field.name] ?? field.default ?? '')"
        class="input"
        @change="setValue(field.name, ($event.target as HTMLSelectElement).value)"
      >
        <option v-for="opt in field.options || []" :key="String(opt.value)" :value="String(opt.value)">
          {{ opt.label }}
        </option>
      </select>

      <!-- 多选 -->
      <div v-else-if="field.type === 'multi-select'" class="multi-select">
        <template v-if="field.options && field.options.length">
          <label v-for="opt in field.options" :key="String(opt.value)" class="checkbox">
            <input
              type="checkbox"
              :checked="isArraySelected(field.name, opt.value)"
              @change="toggleArrayValue(field.name, opt.value, ($event.target as HTMLInputElement).checked)"
            />
            <span>{{ opt.label }}</span>
          </label>
        </template>
        <template v-else>
          <input
            :value="arrayToText(model[field.name])"
            type="text"
            class="input"
            :placeholder="field.placeholder || '逗号分隔多个值'"
            @input="setValue(field.name, parseArray(($event.target as HTMLInputElement).value))"
          />
        </template>
      </div>

      <!-- 文本域 -->
      <textarea
        v-else-if="field.type === 'textarea'"
        :value="arrayToText(model[field.name])"
        class="input textarea"
        rows="2"
        :placeholder="field.placeholder || ''"
        @input="setValue(field.name, parseArray(($event.target as HTMLTextAreaElement).value))"
      ></textarea>

      <p v-if="field.description" class="field-desc">{{ field.description }}</p>
    </div>
  </div>
</template>

<style scoped>
.task-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.form-label {
  font-size: 12px;
  color: var(--text-secondary);
  font-weight: 500;
}

.required {
  color: var(--danger);
}

.field-desc {
  font-size: 11px;
  color: var(--text-secondary);
  opacity: 0.8;
}

/* 开关样式 */
.switch {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}

.switch input {
  display: none;
}

.switch-slider {
  width: 36px;
  height: 20px;
  background: var(--border);
  border-radius: 10px;
  position: relative;
  transition: background 0.2s;
}

.switch-slider::before {
  content: '';
  position: absolute;
  width: 16px;
  height: 16px;
  background: #fff;
  border-radius: 50%;
  top: 2px;
  left: 2px;
  transition: transform 0.2s;
}

.switch input:checked + .switch-slider {
  background: var(--primary);
}

.switch input:checked + .switch-slider::before {
  transform: translateX(16px);
}

.switch-text {
  font-size: 13px;
}

/* 多选 */
.multi-select {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.checkbox {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
  user-select: none;
}

.checkbox:has(input:checked) {
  background: var(--primary);
  color: #fff;
  border-color: var(--primary);
}

.checkbox input {
  display: none;
}

.textarea {
  resize: vertical;
  font-family: monospace;
  font-size: 12px;
}
</style>
