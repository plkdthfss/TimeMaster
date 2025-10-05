import { ref, onMounted, onUnmounted } from 'vue'
import dayjs from 'dayjs'

export function useClock() {
  const currentTime = ref('')
  const currentDate = ref('')
  let timer = null

  const updateTime = () => {
    const now = dayjs()
    currentTime.value = now.format('HH:mm:ss')
    currentDate.value = now.format('YYYY-MM-DD')
  }

  onMounted(() => {
    updateTime()
    timer = setInterval(updateTime, 1000)
  })

  onUnmounted(() => clearInterval(timer))

  return { currentTime, currentDate }
}
