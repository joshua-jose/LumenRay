import socket
import matplotlib.pyplot as plt
import time
import numpy as np

HOST = "127.0.0.1"  # Standard loopback interface address (localhost)
PORT = 65432  # Port to listen on (non-privileged ports are > 1023)

plt.ion() # <-- work in "interactive mode"
fig, ax = plt.subplots()
fig.canvas.set_window_title('LumenRay Metrics')

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
		s.bind((HOST, PORT))
		s.listen()
		while True:
				frame_times = []
				try:
					conn, addr = s.accept()
				except KeyboardInterrupt:
					break
				with conn:
					while True:
						datas = conn.recv(1024*72)
						if not datas:
							break
						for data in [datas[i:i + 16] for i in range(0, len(datas), 16)]:
							frame_time_ns = int.from_bytes(data, "big")
							frame_time_ms = frame_time_ns / 1e6
							#print(f"{frame_time_ms}ms")
							frame_times.append(frame_time_ms)
						
						ax.clear()
						ax.hist(frame_times,bins=40,lw=1,ec="yellow",fc="green",alpha=0.5, range = np.percentile(frame_times,[1,99]))
						
						plt.show()
						plt.pause(0.01)
						#time.sleep(0.01)
	
