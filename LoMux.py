#!/usr/bin/env python3
import os
import platform
import subprocess
import sys
import threading
import shutil
from pathlib import Path
import tkinter as tk
import tkinter.font as tkfont
from tkinter import ttk, filedialog, scrolledtext, messagebox
from ttkthemes import ThemedStyle
from rich.console import Console
import platform

console = Console()

class ToolTip:
	def __init__(self, widget, text, delay=300):
		self.widget=widget
		self.text=text
		self.delay=delay
		self.id=None
		self.tw=None

		widget.bind("<Enter>", self._schedule)
		widget.bind("<Leave>", self._unschedule_and_hide)

	def _schedule(self, _e=None):
		self._unschedule()
		self.id=self.widget.after(self.delay, self._show)

	def _unschedule(self):
		if self.id:
			self.widget.after_cancel(self.id)
			self.id=None

	def _unschedule_and_hide(self, _: tk.Event=None):
		self._unschedule()
		self._hide()

	def _show(self):
		if self.tw or not self.text:
			return
		x = self.widget.winfo_rootx() + 10
		y = self.widget.winfo_rooty() + self.widget.winfo_height() + 5

		tw = self.tw = tk.Toplevel(self.widget)
		tw.wm_overrideredirect(True)
		tw.wm_attributes("-topmost", True)
		tw.wm_geometry(f"+{x}+{y}")

		lbl = tk.Label(tw, text=self.text, justify="left",
						background="#ffffe0", foreground="#000000", 
						relief="solid", borderwidth=1, wraplength=250)
		lbl.pack(ipadx=4, ipady=2)

	def _hide(self):
		if self.tw:
			self.tw.destroy()
			self.tw = None

# find ffmpeg/ffprobe
def find_bin(name: str) -> str:
	
	# try sys path
	system_path = shutil.which(name)
	if system_path:
		return system_path

	# fallback on bundled
	sysname = platform.system().lower()
	subdir = 'windows' if sysname == 'windows' \
			else 'mac' if sysname == 'darwin' \
			else 'linux'
	exe = name + ('.exe' if sysname == 'windows' else '')
	if getattr(sys, 'frozen', False):
		base = Path(sys._MEIPASS)
	else:
		base = Path(__file__).parent

	bundled = base / 'bin' / subdir / exe
	if bundled.exists() and os.access(bundled, os.X_OK):
		return str(bundled)

	# nothing found
	raise FileNotFoundError(f"{name!r} not found on PATH or in {bundled}")

class LoMux(tk.Tk):
	def __init__(self):
		super().__init__()
		self.title("LoMux")
		self.geometry("1000x650")
		self.resizable(True, True)
		self.option_add("*Font", tkfont.nametofont("TkDefaultFont"))
		ttk.Style(self).configure(".", font=tkfont.nametofont("TkDefaultFont"))

		# ThemedStyle for light/dark
		self.style = ThemedStyle(self)
		self._setup_menu_and_theme()

		# Binaries
		try:
			self.ffmpeg = find_bin('ffmpeg')
			self.ffprobe = find_bin('ffprobe')
		except FileNotFoundError as e:
			console.print(f"[red]Error:[/] {e}", style="bold red")
			sys.exit(1)

		# State vars
		self.inputs = []
		self.outdir = None
		self.vbit = tk.IntVar(value=1000)
		self.abit = tk.IntVar(value=128)
		self.flac = tk.IntVar(value=5)
		self.fps = tk.IntVar(value=10)
		self.width = tk.IntVar(value=320)

		self.create_widgets()
		self.update_start_state()

	def _setup_menu_and_theme(self):
		# System / manual theme
		self.apply_system_theme()
		menubar = tk.Menu(self)
		view = tk.Menu(menubar, tearoff=0)
		view.add_command(label="Light Theme", command=self.set_light_theme)
		view.add_command(label="Dark Theme", command=self.set_dark_theme)
		view.add_separator()
		view.add_command(label="Use System Theme", command=self.apply_system_theme)
		menubar.add_cascade(label="View", menu=view)
		self.config(menu=menubar)

	def set_light_theme(self):
		self.style.set_theme('arc')
		self.apply_colors()

	def set_dark_theme(self):
		self.style.set_theme('equilux')
		self.apply_colors()

	def apply_system_theme(self):
		sys_name = platform.system()
		theme = 'equilux'
		if sys_name == 'Darwin':
			try:
				out = subprocess.check_output([
					'defaults', 'read', '-g', 'AppleInterfaceStyle'
				], stderr=subprocess.DEVNULL).strip().decode()
				theme = 'equilux' if out == 'Dark' else 'arc'
			except:
				theme = 'arc'
		elif sys_name == 'Windows':
			theme = 'arc'
		else:
			theme = 'equilux'
		self.style.set_theme(theme)
		self.apply_colors()

	def apply_colors(self):
		# Fetch colors
		self.bg = self.style.lookup('TFrame', 'background')
		self.fg = self.style.lookup('TLabel', 'foreground')
		self.configure(bg=self.bg)
		# Style widgets
		self.style.configure('TLabelframe', background=self.bg)
		self.style.configure('TLabelframe.Label', background=self.bg, foreground=self.fg)
		self.style.configure('TNotebook', background=self.bg)
		self.style.configure('TNotebook.Tab', background=self.bg, foreground=self.fg)
		self.style.map('TNotebook.Tab', background=[('selected', self.bg)], foreground=[('selected', self.fg)])
		self.style.configure('TCombobox', fieldbackground=self.bg, background=self.bg, foreground=self.fg)
		self.style.map('TCombobox', selectbackground=[('readonly', self.bg)], selectforeground=[('readonly', self.fg)])
		self.style.configure('TEntry', fieldbackground=self.bg, background=self.bg, foreground=self.fg)
		self.style.configure('TButton', background=self.bg, foreground=self.fg)
		self.style.configure('Horizontal.TProgressbar', troughcolor=self.bg, background=self.fg)
				# Recursive style only for tk widgets
		def rec(widget):
			for c in widget.winfo_children():
				try:
					if isinstance(c, (tk.Frame, tk.LabelFrame)):
						c.config(bg=self.bg)
					if isinstance(c, tk.Label):
						c.config(bg=self.bg, fg=self.fg)
				except tk.TclError:
					pass
				rec(c)
		rec(self)

	def create_widgets(self):
		self.grid_rowconfigure(0, weight=1)
		self.grid_columnconfigure(1, weight=1)
		controls = tk.Frame(self, bg=self.bg)
		controls.grid(row=0, column=0, sticky='ns', padx=10, pady=10)
		# Tabs
		notebook = ttk.Notebook(controls)
		basic = tk.Frame(notebook, bg=self.bg)
		adv = tk.Frame(notebook, bg=self.bg)
		notebook.add(basic, text='Basic')
		notebook.add(adv, text='Advanced')
		notebook.pack(fill='x', pady=5)
		# Basic: IO
		io_frame = tk.LabelFrame(basic, text='Input & Output', bg=self.bg, fg=self.fg)
		io_frame.pack(fill='x', pady=5)
		ttk.Button(io_frame, text='Select Files...', command=self.select_files).pack(fill='x', pady=2)
		self.files_var = tk.StringVar(master=self, value='No files selected')
		tk.Label(io_frame, textvariable=self.files_var, bg=self.bg, fg=self.fg).pack(fill='x', pady=2)
		ttk.Button(io_frame, text='Select Output Dir...', command=self.select_outdir).pack(fill='x', pady=2)
		self.outdir_var = tk.StringVar(master=self, value='No output directory')
		tk.Label(io_frame, textvariable=self.outdir_var, bg=self.bg, fg=self.fg).pack(fill='x', pady=2)
		# Basic: Presets & Controls
		pf = tk.LabelFrame(basic, text='Presets & Controls', bg=self.bg, fg=self.fg)
		pf.pack(fill='x', pady=5)
		tk.Label(pf, text='Preset:', bg=self.bg, fg=self.fg).grid(row=0, column=0, sticky='e', padx=5, pady=2)
		self.preset_cb = ttk.Combobox(pf, values=['MP4','MKV','WEBM','MP3','FLAC','GIF'], state='readonly')
		self.preset_cb.current(0)
		self.preset_cb.grid(row=0, column=1, sticky='we', padx=5, pady=2)
		self.preset_cb.bind('<<ComboboxSelected>>', lambda e: self.on_preset_change())
		# Dynamic controls
		self.video_label = tk.Label(pf, text='Video Bitrate:', bg=self.bg, fg=self.fg)
		self.video_spin = ttk.Combobox(pf, values=[250,500,1000,2000,4000], textvariable=self.vbit, state='readonly', width=8)
		ToolTip(self.video_spin, '-b:v bitrate')
		self.audio_label = tk.Label(pf, text='Audio Bitrate:', bg=self.bg, fg=self.fg)
		self.audio_spin = ttk.Combobox(pf, values=[64,96,128,192,256,320], textvariable=self.abit, state='readonly', width=8)
		ToolTip(self.audio_spin, '-b:a bitrate')
		self.flac_label = tk.Label(pf, text='FLAC Level:', bg=self.bg, fg=self.fg)
		self.flac_spin = ttk.Spinbox(pf, from_=0, to=8, textvariable=self.flac, width=5)
		ToolTip(self.flac_spin, '-compression_level')
		self.fps_label = tk.Label(pf, text='GIF FPS:', bg=self.bg, fg=self.fg)
		self.fps_spin = ttk.Combobox(pf, values=[10,15,24,30,60], textvariable=self.fps, state='readonly', width=5)
		ToolTip(self.fps_spin, 'Frame rate for GIF')
		self.width_label = tk.Label(pf, text='GIF Width:', bg=self.bg, fg=self.fg)
		self.width_spin = ttk.Combobox(pf, values=[320,480,640,800,1024], textvariable=self.width, state='readonly', width=5)
		ToolTip(self.width_spin, 'Width for GIF, preserves aspect ratio')
		# Advanced tab
		ttk.Label(adv, text='Extra FFmpeg Args:').pack(anchor='w', padx=5, pady=2)
		self.extra_entry = ttk.Entry(adv, width=40)
		self.extra_entry.pack(fill='x', padx=5, pady=2)
		ToolTip(self.extra_entry, 'Raw ffmpeg flags')
		# Controls bottom
		self.run_btn = ttk.Button(controls, text='Start Processing', command=self.start_processing)
		self.run_btn.pack(fill='x', pady=10)
		self.progress = ttk.Progressbar(controls, orient='horizontal', mode='determinate', maximum=100)
		self.progress.pack(fill='x', pady=5)
		self.status = tk.Label(self, text='Ready', relief='sunken', anchor='w', bg=self.bg, fg=self.fg)
		self.status.grid(row=1, column=0, columnspan=2, sticky='we')
		# Console log
		log_frame = tk.LabelFrame(self, text='Console Output', bg=self.bg, fg=self.fg)
		log_frame.grid(row=0, column=1, sticky='nsew', padx=10, pady=10)
		log_frame.rowconfigure(0, weight=1)
		log_frame.columnconfigure(0, weight=1)
		self.log_widget = scrolledtext.ScrolledText(log_frame, state='disabled', background='#2e2e2e', foreground='white', insertbackground='white')
		self.log_widget.grid(row=0, column=0, sticky='nsew')
		self.on_preset_change()

	def set_progress(self, val, text):
		self.after(0, lambda: self._set_progress(val, text))
	def _set_progress(self, val, text):
		self.progress['value'] = val
		self.status.config(text=text)

	def update_start_state(self):
		state = 'normal' if self.inputs and self.outdir else 'disabled'
		self.run_btn.config(state=state)

	def on_preset_change(self):
		for w in (self.video_label, self.video_spin,
				  self.audio_label, self.audio_spin,
				  self.flac_label, self.flac_spin,
				  self.fps_label, self.fps_spin,
				  self.width_label, self.width_spin):
			try:
				w.grid_forget()
			except:
				pass
		pf = self.preset_cb.get(); row=1
		if pf in ('MP4','MKV','WEBM'):
			self.video_label.grid(row=row, column=0, padx=5, pady=2, sticky='e')
			self.video_spin.grid (row=row, column=1, padx=5, pady=2, sticky='w'); row+=1
			self.audio_label.grid(row=row, column=0, padx=5, pady=2, sticky='e')
			self.audio_spin.grid (row=row, column=1, padx=5, pady=2, sticky='w')
		elif pf=='MP3':
			self.audio_label.grid(row=row, column=0, padx=5, pady=2, sticky='e')
			self.audio_spin.grid (row=row, column=1, padx=5, pady=2, sticky='w')
		elif pf=='FLAC':
			self.flac_label.grid (row=row, column=0, padx=5, pady=2, sticky='e')
			self.flac_spin.grid  (row=row, column=1, padx=5, pady=2, sticky='w')
		elif pf=='GIF':
			self.fps_label.grid   (row=row, column=0, padx=5, pady=2, sticky='e')
			self.fps_spin.grid    (row=row, column=1, padx=5, pady=2, sticky='w'); row+=1
			self.width_label.grid (row=row, column=0, padx=5, pady=2, sticky='e')
			self.width_spin.grid  (row=row, column=1, padx=5, pady=2, sticky='w')
		self.update_start_state()

	def select_files(self):
		files = filedialog.askopenfilenames(title='Select Media Files')
		if files:
			self.inputs = list(files)
			self.files_var.set("\n".join(Path(f).name for f in files))
		self.update_start_state()

	def select_outdir(self):
		d = filedialog.askdirectory(title='Select Output Directory')
		if d:
			self.outdir = d
			self.outdir_var.set(Path(d).name)
		self.update_start_state()

	def start_processing(self):
		threading.Thread(target=self._process, daemon=True).start()

	def _process(self):
		total = len(self.inputs)
		for idx, inp in enumerate(self.inputs, start=1):
			self.set_progress(0, f'Processing {idx}/{total}...')
			dur = 0
			if self.ffprobe:
				try:
					out = subprocess.check_output([
						self.ffprobe, '-v', 'error',
						'-show_entries', 'format=duration',
						'-of', 'csv=p=0', inp
					])
					dur = float(out.strip())
				except:
					dur = 0
			cmd = [self.ffmpeg, '-hide_banner', '-threads', '2', '-progress', 'pipe:1', '-nostats', '-i', inp]
			pf = self.preset_cb.get()
			vb = f"{self.vbit.get()}k"; ab = f"{self.abit.get()}k"
			if pf == 'MP4': cmd += ['-c:v','libx264','-b:v',vb,'-c:a','aac','-b:a',ab]
			elif pf == 'MKV': cmd += ['-c:v','libx265','-b:v',vb,'-c:a','libopus','-b:a',ab]
			elif pf == 'WEBM':cmd += ['-c:v','libvpx-vp9','-b:v',vb,'-c:a','libopus','-b:a',ab]
			elif pf == 'MP3': cmd += ['-vn','-c:a','libmp3lame','-b:a',ab]
			elif pf == 'FLAC':cmd += ['-vn','-c:a','flac','-compression_level',str(self.flac.get())]
			elif pf == 'GIF': cmd += ['-vf',f"fps={self.fps.get()},scale={self.width.get()}:-1:flags=lanczos"]
			extra = self.extra_entry.get().strip()
			if extra: cmd += extra.split()
			out_file = Path(self.outdir) / f"{Path(inp).stem}_{pf.lower()}.{pf.lower()}"
			cmd += ['-y', str(out_file)]

			proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
			self.log_widget.config(state='normal')
			self.log_widget.insert(tk.END, f"\n=== Converting {Path(inp).name} ({idx}/{total}) ===\n")
			self.log_widget.see(tk.END)
			self.log_widget.config(state='disabled')

			for line in proc.stdout:

				if line.strip() == 'progress=end':
					break

				self.log_widget.config(state='normal')
				self.log_widget.insert(tk.END, line)
				self.log_widget.see(tk.END)
				self.log_widget.config(state='disabled')

				stripped = line.strip()
				if stripped.startswith('out_time_ms') and dur > 0:
					parts = stripped.split('=', 1)
					if len(parts) == 2 and parts[1].isdigit():
						ms = int(parts[1])
						pct = min(ms / 1000 / dur * 100, 100)
						self.set_progress(pct, f'Processing {idx}/{total}: {pct:.1f}%')
			proc.wait(
			self.set_progress(100, f'Finished {Path(inp).name}')
)
		self.set_progress(100, 'All tasks complete!')
		self.log_widget.config(state='normal')
		self.log_widget.insert(tk.END, 'All tasks complete!\n')
		self.log_widget.see(tk.END)
		self.log_widget.config(state='disabled')
		self.run_btn.config(state='disabled')

if __name__ == '__main__':
	app = LoMux()
	app.mainloop()
