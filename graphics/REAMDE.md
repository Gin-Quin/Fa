The Fa language has several goals :

- To be readable : that should not be such a great effort to read Fa
- To be minimalistic : every *"useless"* the syntax is removed
- To be interpreted or natively compiled : for maximum speed and efficency.
- To be close to its target languages : Fa's goal is not to be interpreted or compiled by itself (since there is already so much great work that has been done with C++ and JS). But the transpilation should be as pure as possible.

But also :

- To facilitate **creative programming**.

Humans mostly use two types of visual software :

- Interactive documents : a user interface to create or manipulate data. A lot of efforts are put into displaying text and capturing user's input. Also there is a system of layouts to organize the interactive document depending on the screen size.
- Intense graphical applications like games.

Depending on the type of the application we want to create, we have different needs. Either a strong support for text and input is needed, or a strong 2D/3D rendering engine with physical rules and all the stuff.

Nowadays, the easiest way to create an interactive document is the web technologies (JS + a modern framework like Svelte, Vue or React). Targetting web technologies with Fa seems legit : the future will be build on top of those. Actually the "web" technologies become less and less web-only.

But I also want to use the power of native Fa. So I questioned myself : should I create a cool high-performance graphical library for Fa? Meaning : a standard library, closely connected to the language itself?

After many reflexion, my answer is no. For several reasons :

- First, the graphic APIs are currently having a small revolution with Vulkan and Metal slowly taking out OpenGL and his friends. I'd later wait how this graphic API war is going to end.
- There are many graphical frameworks, and all of them have their own area of specialisation. I could put a lot of effort into building a standard Fa's framework. But I could also put almost zero effort into building no framework but easy bindings and the everyone would use his framework of heart.

What I could do is propose my own web framework inspired by Svelte and React with natural bindings :

- with a NativeScript-like framework for mobile applications.
- with an Electron-like framework for desktop applications.

But that's actually a different project.