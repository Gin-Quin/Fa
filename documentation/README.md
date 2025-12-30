Fa's main advantages lie in:

- Possibility of mixing interpreted and compiled code ("TypeScript at the speed of Rust")
- A sound and simple memory management system (leveraging relations, containers, copyable objects, aliases, **owned references** and immutable references). Among all those techniques, the most powerful is **owned references**, invented by Fa.

The Fa language has several goals:

- To be readable
- To be minimalistic
- To be **interpreted** or **natively compiled**: for maximum speed and efficiency.
- To be close to its target languages: Fa's goal is not to be interpreted or compiled by itself (since there is already so much great work that has been done with Rust and JS). But the transpilation should be as pure as possible.

But also:

- To facilitate **creative programming**.

Humans mostly use two types of visual software:

- Interactive documents: a user interface to create or manipulate data. A lot of effort is put into displaying text and capturing user input. Also, there is a system of layouts to organize the interactive document depending on the screen size.
- Intense graphical applications like games.

Depending on the type of application we want to create, we have different needs. Either strong support for text and input is needed, or a strong 2D/3D rendering engine with physical rules and all the stuff.

Nowadays, the easiest way to create an interactive document is web technologies (JS + a modern framework like Svelte, Vue or React). Targeting web technologies with Fa seems legit: the future will be built on top of those. Actually, "web" technologies are becoming less and less web-only.

But I also want to use the power of native Fa. So I asked myself: should I create a cool high-performance graphical library for Fa? Meaning: a standard library, closely connected to the language itself?

After much reflection, my answer is no. For several reasons:

- First, the graphics APIs are currently having a small revolution with Vulkan and Metal slowly taking out OpenGL and its friends. I'd rather wait to see how this graphics API war is going to end and wait for the next very good universal framework (the next SDL or SFML),
- There are many graphical frameworks, and all of them have their own area of specialisation. I could put a lot of effort into building a standard Fa framework. But I could also put almost zero effort into building no framework but easy bindings, and then everyone would use their framework of choice.

Plus I really think the future of programming is not those kinds of frameworks. I think the future is a webview-like system:

- Every client device has its functional webview (of course, otherwise they could not navigate the web),
- A webview is less and less "web" and more and more "universal view". You can create any application with a webview,
- So an application will be JavaScript/Wasm, directly connected to the webview,
- The file system modifications (saving, etc...) would be done via a remote server (web-style) or a local server (Electron-style).

The near future is JS and the upcoming future is Wasm. So let's focus on these two targets. And also on Rust because it's so cool and powerful.

I may create my own graphical framework later when Wasm is mature and ready. Fa -> Wasm -> (Web)View API. (Possible to reuse a Rust Wasm framework.)
