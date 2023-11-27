<script lang="ts">
import { defineComponent } from "vue";
import ClockDate from "./components/ClockDate.vue";
import ClockTime from "./components/ClockTime.vue";

export default defineComponent({
  name: "ClockWidget",
  data() {
    return {
      day: 0,
      month: 0,
      year: 0,
      min: "00",
      hour: "00",
      seconds: "00",
      amPm: "AM",
    };
  },
  methods: {
    setTime() {
      const date = new Date();
      let hour = date.getHours(); // 0 - 23
      let min = date.getMinutes(); // 0 - 59
      let seconds = date.getSeconds(); // 0 - 59
      this.day = date.getDate();
      this.month = date.getMonth() + 1;
      this.year = date.getFullYear();

      if (hour > 12) {
        hour = hour - 12;
        this.amPm = "PM";
      } else {
        this.amPm = "AM";
      }

      if (hour == 0) {
        hour = 12;
      }

      this.hour = hour < 10 ? `0${hour}` : `${hour}`;
      this.min = min < 10 ? `0${min}` : `${min}`;
      this.seconds = seconds < 10 ? `0${seconds}` : `${seconds}`;
    },
  },
  computed:{
    cDay(): number{
      return this.day
    },
    cMonth(): number{
      return this.month
    },
    cYear(): number{
      return this.year
    },
    cHour(): string{
      return this.hour
    },
    cMin(): string{
      return this.min
    },
    cSeconds(): string{
      return this.seconds
    },
    cAmPm(): string{
      return this.amPm
    }
  },
  mounted() {
    setInterval(() => this.setTime(), 1000);
  },
  components: {
    ClockDate,
    ClockTime,
  },
});
</script>

<template>
  <div class="clock-warp">
    <div class="clock-widget">
      <ClockDate :day="cDay" :month="cMonth" :year="cYear" />
      <ClockTime :hours="cHour" :minutes="cMin" :amPm="cAmPm" :seconds="cSeconds" />
    </div>
  </div>
</template>
