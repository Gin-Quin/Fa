


<div id={stage.name} class="editor">
let x = 12
let y: String = "Hey"

add(x: Integer, y: Integer) = 12
</div>


<script>
	import { onMount } from 'svelte'

	const mode = {
		'Fa': 'nim',
		'Javascript': 'javascript',
		'Typescript': 'typescript',
		'C++': 'c_cpp',
	}
	
	export let stage
	let editor = null

	onMount(() => {
		console.log("ON MOUNT", document.getElementById(stage.name))

		editor = ace.edit(stage.name, {
			mode: "ace/mode/nim",
			behavioursEnabled: false,
			showPrintMargin: false,
			showGutter: false,
			tabSize: 3,
			useSoftTabs: false,
			readOnly: stage.readonly,
			
			theme: "ace/theme/monokai"
			// theme: "ace/theme/xcode"
			// theme: "ace/theme/dawn"
			// theme: "ace/theme/dracula"
		})
	})

	$: editor?.session.setMode(`ace/mode/${mode[stage.name]}`)

</script>


<style lang=sass>
	.editor
		width: 100%
		height: 100%
</style>