import matplotlib.pyplot as plt
import matplotlib.animation as animation
from matplotlib import style
import numpy as np

style.use('fivethirtyeight')

fig, ax = plt.subplots()

def animate(i):
	frame_times = []
	with open("metrics.csv", 'r') as f:
		for data in f.readlines():
			frame_time_ns = int(data)
			frame_time_ms = frame_time_ns / 1e6
			#print(f"{frame_time_ms}ms")
			frame_times.append(frame_time_ms)

	if len(frame_times) == 0:
		return
	
	ax.clear()
	ax.hist(frame_times,bins=40,lw=1,ec="yellow",fc="green",alpha=0.5, range = np.percentile(frame_times,[1,99]))


ani = animation.FuncAnimation(fig, animate, interval=100)
# animate(1)
plt.show()