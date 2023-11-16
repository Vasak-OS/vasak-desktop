const ClockWidget = {
	data() {
		return {
			day: 0,
			month: 0,
			year: 0,
			min: '00',
			hour: '00',
			seconds: '00',
			amPm: 'AM',
		};
	},
	mounted() {
		setInterval(() => this.setTime(), 1000);
	},
	methods: {
		setTime() {
			const date = new Date();
			var hour = date.getHours(); // 0 - 23
			var min = date.getMinutes(); // 0 - 59
			var seconds = date.getSeconds(); // 0 - 59
			this.day = date.getDate();
			this.month = date.getMonth() + 1;
			this.year = date.getFullYear();

			if (hour > 12) {
				hour = hour - 12;
				this.amPm = 'PM';
			} else {
				this.amPm = 'AM';
			}

			if (hour == 0) {
				hour = 12;
			}

			this.hour = hour < 10 ? `0${hour}` : `${hour}`;
			this.min = min < 10 ? `0${min}` : `${min}`;
			this.seconds = seconds < 10 ? `0${seconds}` : `${seconds}`;
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
		ClockTime,
	},
};
