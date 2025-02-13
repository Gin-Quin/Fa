export default function(
	fileName: String,
	value: Any,
	schema: (Schema | SchemaWithMemory)? = null,
) {
	return new Promise((resolve, reject) =>
		open(fileName, mode = "w"): (error, fileDescriptor) ->
			if error: reject(error)
			else: resolve(FileEncoder(fileDescriptor).encode(value, schema))
)