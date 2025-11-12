Local photos for the quiz

Place real, free-to-use photos in `public/assets/` with the following filenames (examples):

- `tanuki1.jpg`, `tanuki2.jpg`, `tanuki3.jpg`
- `anaguma1.jpg`, `anaguma2.jpg`, `anaguma3.jpg`
- `hakubishin1.jpg`, `hakubishin2.jpg`, `hakubishin3.jpg`

If these files exist, the server will use them for the quiz and return local URLs like `/assets/tanuki1.jpg`.
If a category has no local file, the server will fall back to Unsplash Source URLs.

Quick PowerShell example to download sample images (replace with properly licensed photos for production):

```powershell
mkdir -Force public\assets
Invoke-WebRequest -OutFile public\assets\tanuki1.jpg -Uri "https://source.unsplash.com/800x600/?tanuki" -UseBasicParsing
Invoke-WebRequest -OutFile public\assets\anaguma1.jpg -Uri "https://source.unsplash.com/800x600/?badger" -UseBasicParsing
Invoke-WebRequest -OutFile public\assets\hakubishin1.jpg -Uri "https://source.unsplash.com/800x600/?civet" -UseBasicParsing
```

Note: Unsplash images are free to use but check license and attribution requirements if you publish the site.
