Fa's main advantages lay in:

- Possibility to mix interpreted vs compiled ("Typescript at the speed of Rust")
- A sound and simple memory management system (leveraging relations, containers, copiable objects, aliases, **owned references** and immutable references). Among all those techniques, the most powerful is **owned references**, invented by Fa.

The Fa language has several goals:

- To be readable
- To be minimalistic
- To be **interpreted** or **natively compiled**: for maximum speed and efficency.
- To be close to its target languages: Fa's goal is not to be interpreted or compiled by itself (since there is already so much great work that has been done with Rust and JS). But the transpilation should be as pure as possible.

But also:

- To facilitate **creative programming**.

Humans mostly use two types of visual software:

- Interactive documents: a user interface to create or manipulate data. A lot of efforts are put into displaying text and capturing user's input. Also there is a system of layouts to organize the interactive document depending on the screen size.
- Intense graphical applications like games.

Depending on the type of the application we want to create, we have different needs. Either a strong support for text and input is needed, or a strong 2D/3D rendering engine with physical rules and all the stuff.

Nowadays, the easiest way to create an interactive document is the web technologies (JS + a modern framework like Svelte, Vue or React). Targeting web technologies with Fa seems legit: the future will be build on top of those. Actually the "web" technologies become less and less web-only.

But I also want to use the power of native Fa. So I questioned myself: should I create a cool high-performance graphical library for Fa? Meaning: a standard library, closely connected to the language itself?

After many reflexion, my answer is no. For several reasons:

- First, the graphic APIs are currently having a small revolution with Vulkan and Metal slowly taking out OpenGL and his friends. I'd later wait how this graphic API war is going to end and wait for the next very good universal framework (the next SDL or SFML),
- There are many graphical frameworks, and all of them have their own area of specialisation. I could put a lot of effort into building a standard Fa's framework. But I could also put almost zero effort into building no framework but easy bindings and the everyone would use his framework of heart.

Plus I really think the future of programming is not those kind of frameworks. I think the future is a webview-like system:

- Every client device has its functional webview (of course, otherwise they could not navigate the web),
- A webview is less and less "web" and more and more "universal view". You can create any application with a web view,
- So an application will be Javascript/Wasm, directly connected to the webview,
- The file system modifications (saving, etc...) would be done via a distant server (web-style) or a local server (electron-style).

The near future is Js and the upcoming future is Wasm. So let's focus on these two targets. And also on Rust because because it's so cool and powerful.

I may create my own graphical framework later when Wasm is mature and ready. Fa -> Wasm -> (Web)View API. (Possible to reuse a Rust Wasm framework.)
