export default {
	props: {
		hours: {
			type: Number,
			required: true
		},
		minutes: {
			type: Number,
			required: true
		},
		amPm: {
			type: String,
			required: true
		},
		seconds: {
			type: Number,
			required: true
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
	components: {}
};