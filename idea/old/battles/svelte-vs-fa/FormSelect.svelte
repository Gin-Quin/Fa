<script lang="ts">
	import Svelect from "atoms/Svelect.svelte"

	type Item = {
		value: any
		text?: string
	}

	export let label = ""
	export let name: string
	export let value: null | unknown = null
	export let placeholder = "Select an option..."
	export let items: Item[]
</script>

<div class="form-input">
	<label for={name}>{label}</label>
	<Svelect {name} bind:value {placeholder} {items}>
		<div slot="item" let:item>
			<slot name="item" {item}>
				{item.text || item.value}
			</slot>
		</div>
	</Svelect>
</div>
