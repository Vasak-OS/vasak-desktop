const ClockTime = {
	props: {
		hours: {
			type: String,
			required: true,
		},
		minutes: {
			type: String,
			required: true,
		},
		amPm: {
			type: String,
			required: true,
		},
		seconds: {
			type: String,
			required: true,
		},
	},
	template: `
        <div class="clock-time text-center">
            <span class="clock-time-hours">{{ hours }}</span>
            <span>:</span>
            <span class="clock-time-minutes">{{ minutes }}</span>
            <span>:</span>
            <div class="clock-time-box">
                <span class="clock-time-am-pm">{{ amPm }}</span>
                <span class="clock-time-seconds">{{ seconds }}</span>
            </div>
        </div>
    `,
	components: {},
};
