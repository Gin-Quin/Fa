
<script>
	import { Icon } from 'svelte-materialify'

	export let
		value,
		key = '',
		level = -1
	
	let isObject, isArray
	$: isObject = value instanceof Object
	$: isArray = Array.isArray(value)

	let minified = level != -1

	/**
	 * Return the keys in the following order :
	 * - first, keys with a simple value (string, integer)
	 * - second, keys with an object value
	 * - then, numeric keys (0, 1, etc...)
	 */
	function orderedKeys(object) {
		let simpleKeys = []
		let complexKeys = []
		let numericKeys = []
		for (let key in object) {
			if (!isNaN(key))
				numericKeys.push(key)
			else if (object[key] instanceof Object)
				complexKeys.push(key)
			else
				simpleKeys.push(key)
		}
		return [...simpleKeys, ...complexKeys, ...numericKeys]
	}


	function toggleMinify() {
		minified = !minified
	}

	const levelColors = [
		'teal',
		'blue',
		'lighten-3 text-lighten-3 pink',
		'darken-2 text-darken-2 amber',
		'cyan',
		'red',
	]

	const levelColor = levelColors[level % levelColors.length]
	const indentGuideColor = levelColors[(level - 1) % levelColors.length]

</script>


<div class="json-key-value" style={`text-indent: ${9 + level * 18}px;`}>
	<span class={`indent-guide ${indentGuideColor}`}/>

	{#if isObject}
		{#if key}
			<div class="row complex" on:click={toggleMinify}>
				<Icon class={`mdi mdi-chevron-${minified ? 'right' : 'down'} ${levelColor}-text`} size=18px/>
				<span class="key">{key}</span>
			</div>
		{/if}

		<div class=values class:minified={minified}>
			{#each orderedKeys(value) as subkey}
				<svelte:self key={subkey} value={value[subkey]} level={level+1}/>
			{/each}
		</div>
	{:else}
		<div class="row simple">
			{#if isNaN(key)}
				<span class="key simple">{key}</span>
			{:else}
				<Icon class="mdi mdi-minus" size=14px/>
			{/if}
			<span class=value type={typeof value}>{value}</span>
		</div>
	{/if}
</div>


<style lang=sass>
	.json-key-value
		text-align: left
		position: relative

		> .indent-guide
			position: absolute
			top: 0
			width: 1px
			height: 100%
			transform: translateX(-10px)

	.key
		padding: 2px 1px 2px 3px
		border-bottom: 1px solid #aaa
		color: #555
		font-style: italic

		&.simple
			margin: 0 6px

	.values
		&.minified
			display: none

	.value
		flex: 1
		text-align: right

		&[type="string"]
			color: #00bf07
			font-style: italic

		&[type="number"]
			font-weight: bold
			color: #7316ec

		&[type="boolean"]
			font-weight: bold
			color: #d603ce

		&[type="undefined"]
			font-weight: bold
			color: #4fa2d6



	.row
		line-height: 32px
		font-size: 14px

		&.complex:hover
			background: rgba(0, 0, 0, 0.05)

		&.complex
			cursor: pointer
</style>