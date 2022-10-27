export default {
	props: {
		day: {
			type: Number,
			required: true
		},
		month: {
			type: Number,
			required: true
		},
		year: {
			type: Number,
			required: true
		},
	},
	template: `
    <div class="clock-date text-center">
        <span class="clock-date-day">{{ day }}</span>
        <span>/</span>
        <span class="clock-date-month">{{ month }}</span>
        <span>/</span>
        <span class="clock-date-year">{{ year }}</span>
    </div>
    `
};