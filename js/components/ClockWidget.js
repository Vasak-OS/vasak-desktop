import ClockDate from './clock/ClockDate.js';
import ClockTime from './clock/ClockTime.js';

export default {
	data() {
		return {
			day: 0,
			month: 0,
			year: 0,
			min: 0,
			hour: 0,
			seconds: 0,
			amPm: 'AM',
		};
	},
	mounted() {
		setInterval(() => this.setTime(), 1000);
	},
	methods: {
		setTime() {
			const date = new Date();
			this.hour = date.getHours(); // 0 - 23
			this.min = date.getMinutes(); // 0 - 59
			this.seconds = date.getSeconds(); // 0 - 59
			this.day = date.getDate();
			this.month = date.getMonth() + 1;
			this.year = date.getFullYear();

			if (this.hour > 12) {
				this.hour = this.hour - 12;
				this.amPm = 'PM';
			} else {
				this.amPm = 'AM';
			}

			if (this.hour == 0) {
				this.hour = 12;
			}

			this.hour = this.hour < 10 ? `0${this.hour}` : this.hour;
			this.min = this.min < 10 ? `0${this.min}` : this.min;
			this.seconds = this.seconds < 10 ? `0${this.seconds}` : this.seconds;
		},
	}, 
	template: `
    <div class="clock-warp">
        <div class="clock-widget">
            <ClockDate :day="day" :month="month" :year="year" />
            <ClockTime :hours="hour" :minutes="min" :amPm="amPm" :seconds="seconds" />
        </div>
    </div>
    `,
	components: {
		ClockDate,
		ClockTime
	}
};