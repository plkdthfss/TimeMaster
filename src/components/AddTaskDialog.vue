<template>
  <el-dialog
    v-model="visible"
    :title="dialogTitle"
    width="520px"
    destroy-on-close
  >
    <el-form ref="formRef" :model="form" :rules="rules" label-width="88px">
      <el-form-item label="任务名称" prop="name">
        <el-input v-model="form.name" placeholder="例如：整理周报" maxlength="40" show-word-limit />
      </el-form-item>

      <el-form-item label="任务描述" prop="description">
        <el-input
          v-model="form.description"
          type="textarea"
          :autosize="{ minRows: 3, maxRows: 5 }"
          maxlength="120"
          show-word-limit
          placeholder="补充任务细节、验收标准等信息"
        />
      </el-form-item>

      <el-form-item label="任务类型" prop="type">
        <el-radio-group v-model="form.type">
          <el-radio-button label="once">一次性</el-radio-button>
          <el-radio-button label="cycle">周期性</el-radio-button>
          <el-radio-button label="long_term">长期</el-radio-button>
        </el-radio-group>
      </el-form-item>

      <el-form-item label="进度目标" prop="target">
        <el-input-number v-model="form.target" :min="1" />
      </el-form-item>

      <el-form-item v-if="form.type === 'cycle'" label="重复周期" prop="repeat">
        <el-select v-model="form.repeat" placeholder="选择周期">
          <el-option label="每天" value="daily" />
          <el-option label="每周" value="weekly" />
          <el-option label="每月" value="monthly" />
        </el-select>
      </el-form-item>

      <el-form-item v-if="form.type === 'long_term'" label="时间范围" prop="dateRange">
        <el-date-picker
          v-model="form.dateRange"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          value-format="YYYY-MM-DD"
        />
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="handleCancel">取消</el-button>
      <el-button type="primary" @click="handleSave">{{ saveButtonLabel }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup>
import { computed, reactive, ref, watch } from "vue"

const props = defineProps({
  modelValue: {
    type: Boolean,
    default: false
  },
  mode: {
    type: String,
    default: "create"
  },
  task: {
    type: Object,
    default: null
  }
})

const emits = defineEmits(["update:modelValue", "save"])

const formRef = ref(null)
const visible = ref(props.modelValue)

const isEditMode = computed(() => props.mode === "edit")
const dialogTitle = computed(() => (isEditMode.value ? "编辑任务" : "创建新任务"))
const saveButtonLabel = computed(() => (isEditMode.value ? "保存修改" : "保存"))

const createDefaultForm = () => ({
  id: null,
  name: "",
  description: "",
  type: "once",
  target: 1,
  repeat: "daily",
  dateRange: []
})

const toFormModel = (task) => ({
  id: task?.id ?? null,
  name: task?.name ?? "",
  description: task?.description ?? "",
  type: task?.type ?? "once",
  target: task?.target ?? 1,
  repeat: task?.repeatRule ?? "daily",
  dateRange: task?.type === "long_term" && task?.startDate
    ? [task.startDate, task.endDate ?? task.startDate]
    : []
})

const form = reactive(createDefaultForm())

const rules = {
  name: [{ required: true, message: "请输入任务名称", trigger: "blur" }],
  type: [{ required: true, message: "请选择任务类型", trigger: "change" }],
  target: [
    { required: true, message: "请设置进度目标", trigger: "change" },
    {
      validator: (_, value, callback) => {
        if (value <= 0) {
          callback(new Error("目标进度需大于 0"))
        } else {
          callback()
        }
      },
      trigger: "change"
    }
  ],
  repeat: [
    {
      validator: (_, value, callback) => {
        if (form.type === "cycle" && !value) {
          callback(new Error("请选择重复周期"))
        } else {
          callback()
        }
      },
      trigger: "change"
    }
  ],
  dateRange: [
    {
      validator: (_, value, callback) => {
        if (form.type === "long_term") {
          if (!Array.isArray(value) || value.length !== 2) {
            callback(new Error("请选择时间范围"))
            return
          }
        }
        callback()
      },
      trigger: "change"
    }
  ]
}

const syncForm = () => {
  const source = isEditMode.value && props.task ? toFormModel(props.task) : createDefaultForm()
  Object.assign(form, source)
  formRef.value?.clearValidate?.()
}

watch(
  () => props.modelValue,
  (value) => {
    visible.value = value
    if (value) {
      syncForm()
    }
  }
)

watch(visible, (value) => {
  emits("update:modelValue", value)
})

watch(
  () => props.task,
  (value) => {
    if (visible.value && isEditMode.value && value) {
      syncForm()
    }
  }
)

const handleCancel = () => {
  visible.value = false
}

const handleSave = async () => {
  const formInstance = formRef.value
  if (!formInstance) return

  try {
    await formInstance.validate()
    const payload = {
      id: form.id,
      name: form.name,
      description: form.description,
      type: form.type,
      target: form.target,
      repeat: form.repeat,
      dateRange: form.dateRange
    }
    emits("save", payload)
    visible.value = false
  } catch (error) {
    // validation feedback handled by Element Plus
  }
}
</script>

