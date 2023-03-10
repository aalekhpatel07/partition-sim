FROM python:3.9 AS base
RUN apt-get update -y && apt-get upgrade -y
RUN apt-get install curl iptables sudo -y
COPY register_service.py /register_service.py
RUN chmod +x /register_service.py
RUN pip install requests
CMD ["python3", "/register_service.py"]
