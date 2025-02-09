let async export:
	fileName: String
	value: Any
	schema: (Schema | SchemaWithMemory)? = null
=> Promise: (resolve, reject) ->
	open(fileName, mode = "w"): (error, fileDescriptor) ->
		if error: reject(error)
		else: resolve(FileEncoder(fileDescriptor).encode(value, schema))
