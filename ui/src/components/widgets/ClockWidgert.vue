<script lang="ts" setup>
import { computed, onMounted, Ref, ref } from "vue";
import ClockDate from "./components/ClockDate.vue";
import ClockTime from "./components/ClockTime.vue";

const day: Ref<number> = ref(0);
const month: Ref<number> = ref(0);
const year: Ref<number> = ref(0);
const min: Ref<string> = ref("00");
const hour: Ref<string> = ref("00");
const seconds: Ref<string> = ref("00");
const amPm: Ref<string> = ref("AM");

const setTime = () => {
  const date = new Date();
  let dateHour = date.getHours(); // 0 - 23
  let dateMin = date.getMinutes(); // 0 - 59
  let dateSeconds = date.getSeconds(); // 0 - 59
  day.value = date.getDate();
  month.value = date.getMonth() + 1;
  year.value = date.getFullYear();

  if (dateHour > 12) {
    dateHour = dateHour - 12;
    amPm.value = "PM";
  } else {
    amPm.value = "AM";
  }

  if (dateHour == 0) {
    dateHour = 12;
  }

  hour.value = dateHour < 10 ? `0${dateHour}` : `${dateHour}`;
  min.value = dateMin < 10 ? `0${dateMin}` : `${dateMin}`;
  seconds.value = dateSeconds < 10 ? `0${dateSeconds}` : `${dateSeconds}`;
};

const cDay = computed(() => day.value);
const cMonth = computed(() => month.value);
const cYear = computed(() => year.value);
const cHour = computed(() => hour.value);
const cMin = computed(() => min.value);
const cSeconds = computed(() => seconds.value);
const cAmPm = computed(() => amPm.value);

onMounted(() => {
  setInterval(() => setTime(), 1000);
});
</script>

<template>
  <div class="clock-position">
    <div class="widget-frame clock-widget">
      <ClockDate :day="cDay" :month="cMonth" :year="cYear" />
      <ClockTime
        :hours="cHour"
        :minutes="cMin"
        :amPm="cAmPm"
        :seconds="cSeconds"
      />
    </div>
  </div>
</template>
