
<script>
	import { onMount } from 'svelte'
	import JsonKeyValue from './JsonKeyValue.svelte'
	export let stage

	let height = 'auto'

	onMount(() => {
		setHeight()
		document.addEventListener('resize', setHeight, { passive: true })
	})

	function setHeight() {
		let container = document.getElementById(stage.name)
		height = container.parentElement.clientHeight + 'px'
	}
</script>

<div id={stage.name} class="json" style={`height: ${height}`}>
	<JsonKeyValue value={stage.json}/>
</div>

<style lang=sass>
	.json
		width: 100%
		min-height: 100%
		overflow: auto
		background: #efefef
</style>